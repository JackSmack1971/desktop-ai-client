/// SQLite connection management and shell preference persistence.
///
/// All persistence goes through typed backend commands; the frontend never
/// issues raw SQL. WAL mode is enabled for durability during streaming writes.
///
/// Privacy invariant: do not log or return raw SQL values containing user content,
/// prompt text, or credentials.
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::app_state::Surface;

/// Wrapper that holds a `Mutex`-guarded SQLite connection.
/// Tauri manages this as shared state via `tauri::State<'_, SqlitePool>`.
pub struct SqlitePool {
    conn: Mutex<Connection>,
}

impl SqlitePool {
    /// Open or create the SQLite database at `db_path`.
    ///
    /// Enables WAL mode and foreign-key enforcement on every connection.
    /// Applies all pending migrations before returning.
    pub fn open(db_path: PathBuf) -> rusqlite::Result<Self> {
        let conn = Connection::open(&db_path)?;

        // Pragmas required by the architecture spec.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )?;

        // Apply all pending migrations so the schema is up to date before
        // any reads or writes. This fulfills the documented contract above.
        crate::storage::migrations::run_migrations(&conn, env!("CARGO_PKG_VERSION"))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Wrap an already-configured `Connection` in a pool.
    ///
    /// Intended for tests that need to prepare an in-memory database with
    /// specific state (e.g. by running migrations manually) before handing
    /// it to domain stores. Production code should use `open()`.
    pub fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Execute a closure with exclusive access to the connection.
    pub fn with_conn<F, T>(&self, f: F) -> rusqlite::Result<T>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T>,
    {
        let conn = self.conn.lock().unwrap_or_else(|poisoned| {
            // A prior thread panicked while holding the connection lock.
            // The connection state is unknown; propagate the panic rather than
            // misrepresenting a fatal concurrency failure as a parameter error.
            panic!("SQLite connection mutex poisoned: {}", poisoned);
        });
        f(&conn)
    }

    /// Execute a closure inside a single SQLite transaction, committing on
    /// `Ok` and rolling back on `Err` (rusqlite's `Transaction::drop` rolls
    /// back automatically if `commit()` is never called).
    ///
    /// Uses `Connection::unchecked_transaction()` rather than
    /// `Connection::transaction()` because `with_conn`/this method only ever
    /// hands out `&Connection` (the mutex guard is shared, not exclusive at
    /// the type level) — `unchecked_transaction` is rusqlite's documented
    /// escape hatch for exactly this "connection wrapped behind a Mutex"
    /// shape, and the outer `Mutex<Connection>` already prevents any other
    /// caller from interleaving statements while this transaction is open.
    ///
    /// Used wherever multiple tables must change together as one atomic
    /// unit (e.g. persisting assistant output + usage + status + artifact
    /// for a single turn attempt).
    pub fn with_transaction<F, T>(&self, f: F) -> rusqlite::Result<T>
    where
        F: FnOnce(&rusqlite::Transaction) -> rusqlite::Result<T>,
    {
        let conn = self.conn.lock().unwrap_or_else(|poisoned| {
            panic!("SQLite connection mutex poisoned: {}", poisoned);
        });
        let tx = conn.unchecked_transaction()?;
        // Defer FK validation to COMMIT (resets automatically afterwards) so
        // callers can write rows with forward/circular references — e.g. a
        // `turn_attempts.assistant_message_id` update landing before the
        // `messages` row it points to is inserted — in whichever order is
        // most natural, instead of being forced into an FK-satisfying order.
        tx.execute_batch("PRAGMA defer_foreign_keys = ON;")?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }
}

/// A single row from the `conversations` table.
///
/// Serialized to camelCase for IPC responses via `serde::Serialize`.
/// Raw content (title, model) stays backend-owned — the IPC layer maps this
/// to typed response DTOs; the renderer never receives raw SQL row data.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversationRow {
    pub id: String,
    pub title: String,
    pub model: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Typed store for conversations.
///
/// All reads and writes to the `conversations` table must go through this
/// store. IPC handlers must not call `with_conn` directly (anti-pattern in
/// ARCHITECTURE.md).
pub struct ConversationStore {
    pool: std::sync::Arc<SqlitePool>,
}

impl ConversationStore {
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Insert a new conversation row with `status = 'active'`.
    pub fn create_conversation(&self, id: &str, title: &str) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES (?1, ?2)",
                params![id, title],
            )?;
            Ok(())
        })
    }

    /// Return all conversations ordered by `updated_at DESC`.
    pub fn list_conversations(&self) -> rusqlite::Result<Vec<ConversationRow>> {
        self.pool.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, title, model, status, created_at, updated_at
                 FROM conversations
                 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(ConversationRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    model: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row?);
            }
            Ok(result)
        })
    }

    /// Fetch a single conversation by primary key.
    ///
    /// Returns `None` when the conversation does not exist.
    pub fn get_conversation(&self, id: &str) -> rusqlite::Result<Option<ConversationRow>> {
        self.pool.with_conn(|conn| {
            match conn.query_row(
                "SELECT id, title, model, status, created_at, updated_at
                 FROM conversations WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ConversationRow {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        model: row.get(2)?,
                        status: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            ) {
                Ok(row) => Ok(Some(row)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e),
            }
        })
    }

    /// Mark a conversation complete and record the final model identifier.
    ///
    /// Called when `ChatEvent::Done { model }` is received for this conversation.
    pub fn mark_complete(&self, id: &str, model: &str) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "UPDATE conversations
                 SET status = 'complete', model = ?2, updated_at = datetime('now')
                 WHERE id = ?1",
                params![id, model],
            )?;
            Ok(())
        })
    }

    /// Mark a conversation incomplete (cancelled stream).
    pub fn mark_incomplete(&self, id: &str) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "UPDATE conversations
                 SET status = 'incomplete', updated_at = datetime('now')
                 WHERE id = ?1",
                params![id],
            )?;
            Ok(())
        })
    }
}

/// A single row from the `messages` table.
///
/// Serialized to camelCase for IPC responses. Content stays backend-owned;
/// the IPC layer controls what fields cross the Tauri boundary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MessageRow {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

/// Typed store for messages.
///
/// All reads and writes to the `messages` table must go through this store.
/// IPC handlers must not call `with_conn` directly.
pub struct MessageStore {
    pool: std::sync::Arc<SqlitePool>,
}

impl MessageStore {
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Insert a complete message (status = 'complete').
    pub fn insert_message(
        &self,
        id: &str,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content, status)
                 VALUES (?1, ?2, ?3, ?4, 'complete')",
                params![id, conversation_id, role, content],
            )?;
            Ok(())
        })
    }

    /// Insert an incomplete message (status = 'incomplete').
    ///
    /// Used when a stream is cancelled before the assistant message finishes.
    pub fn insert_incomplete_message(
        &self,
        id: &str,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content, status)
                 VALUES (?1, ?2, ?3, ?4, 'incomplete')",
                params![id, conversation_id, role, content],
            )?;
            Ok(())
        })
    }

    /// Return all messages for a conversation ordered by `created_at ASC`.
    pub fn get_messages(&self, conversation_id: &str) -> rusqlite::Result<Vec<MessageRow>> {
        self.pool.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, conversation_id, role, content, status, created_at
                 FROM messages
                 WHERE conversation_id = ?1
                 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map(params![conversation_id], |row| {
                Ok(MessageRow {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?;
            let mut result = Vec::new();
            for row in rows {
                result.push(row?);
            }
            Ok(result)
        })
    }
}

/// Typed store for workspace shell preferences.
///
/// Wraps `SqlitePool` with a domain-specific API so IPC command handlers
/// do not construct raw SQL. This is the only layer allowed to read or
/// write the `shell_preferences` table.
pub struct ShellPreferenceStore {
    pool: std::sync::Arc<SqlitePool>,
}

impl ShellPreferenceStore {
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Persist the active surface. Creates or replaces the singleton row.
    pub fn save_active_surface(&self, surface: &Surface) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO shell_preferences (id, active_surface, updated_at)
                 VALUES (1, ?1, datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                   active_surface = excluded.active_surface,
                   updated_at = excluded.updated_at",
                params![surface.to_string()],
            )?;
            Ok(())
        })
    }

    /// Load the persisted active surface, if any.
    pub fn load_active_surface(&self) -> rusqlite::Result<Option<Surface>> {
        self.pool.with_conn(|conn| {
            let result = conn.query_row(
                "SELECT active_surface FROM shell_preferences WHERE id = 1",
                [],
                |row| row.get::<_, String>(0),
            );

            match result {
                Ok(s) => {
                    let surface = s.parse::<Surface>().map_err(|e| {
                        rusqlite::Error::InvalidParameterName(format!(
                            "stored surface value is invalid: {e}"
                        ))
                    })?;
                    Ok(Some(surface))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::Surface;
    use crate::storage::migrations::run_migrations;

    fn in_memory_pool() -> SqlitePool {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        SqlitePool {
            conn: Mutex::new(conn),
        }
    }

    fn migrated_pool() -> std::sync::Arc<SqlitePool> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        run_migrations(&conn, "0.0.0-test").unwrap();
        std::sync::Arc::new(SqlitePool::from_connection(conn))
    }

    #[test]
    fn save_and_load_active_surface_round_trips() {
        let pool = std::sync::Arc::new(in_memory_pool());

        // Apply the shell_preferences schema inline for the test.
        pool.with_conn(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS shell_preferences (
                   id INTEGER PRIMARY KEY CHECK (id = 1),
                   active_surface TEXT NOT NULL,
                   updated_at TEXT NOT NULL
                 );",
            )
        })
        .unwrap();

        let store = ShellPreferenceStore::new(pool);

        // No row yet should return None.
        assert_eq!(store.load_active_surface().unwrap(), None);

        // Save and reload.
        store.save_active_surface(&Surface::History).unwrap();
        assert_eq!(store.load_active_surface().unwrap(), Some(Surface::History));

        // Overwrite and confirm the new value is returned.
        store.save_active_surface(&Surface::Artifacts).unwrap();
        assert_eq!(
            store.load_active_surface().unwrap(),
            Some(Surface::Artifacts)
        );
    }

    #[test]
    fn conversation_store_create_list_get_round_trip() {
        let pool = migrated_pool();
        let store = ConversationStore::new(pool);

        // Create a conversation.
        store.create_conversation("conv-a", "First Chat").unwrap();

        // List should return it.
        let list = store.list_conversations().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "conv-a");
        assert_eq!(list[0].title, "First Chat");
        assert_eq!(list[0].status, "active");

        // Get by id.
        let conv = store.get_conversation("conv-a").unwrap();
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().title, "First Chat");

        // Get absent id returns None.
        let absent = store.get_conversation("does-not-exist").unwrap();
        assert!(absent.is_none());
    }

    #[test]
    fn conversation_store_mark_complete_and_incomplete() {
        let pool = migrated_pool();
        let store = ConversationStore::new(pool);

        store.create_conversation("conv-b", "Status Chat").unwrap();

        // Mark complete with model.
        store.mark_complete("conv-b", "claude-sonnet-4-6").unwrap();
        let conv = store.get_conversation("conv-b").unwrap().unwrap();
        assert_eq!(conv.status, "complete");
        assert_eq!(conv.model, "claude-sonnet-4-6");

        // Mark incomplete.
        store.mark_incomplete("conv-b").unwrap();
        let conv = store.get_conversation("conv-b").unwrap().unwrap();
        assert_eq!(conv.status, "incomplete");
    }

    #[test]
    fn message_store_insert_and_get_messages_round_trip() {
        let pool = migrated_pool();
        let conv_store = ConversationStore::new(pool.clone());
        let msg_store = MessageStore::new(pool);

        conv_store
            .create_conversation("conv-c", "Msg Chat")
            .unwrap();

        msg_store
            .insert_message("msg-1", "conv-c", "user", "Hello")
            .unwrap();
        msg_store
            .insert_message("msg-2", "conv-c", "assistant", "Hi there")
            .unwrap();
        msg_store
            .insert_incomplete_message("msg-3", "conv-c", "assistant", "partial…")
            .unwrap();

        let messages = msg_store.get_messages("conv-c").unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi there");
        assert_eq!(messages[1].status, "complete");
        assert_eq!(messages[2].status, "incomplete");
    }
}
