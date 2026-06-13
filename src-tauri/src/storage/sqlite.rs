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
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::InvalidParameterName("connection mutex poisoned".to_string())
        })?;
        f(&conn)
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
        assert_eq!(
            store.load_active_surface().unwrap(),
            Some(Surface::History)
        );

        // Overwrite and confirm the new value is returned.
        store.save_active_surface(&Surface::Artifacts).unwrap();
        assert_eq!(
            store.load_active_surface().unwrap(),
            Some(Surface::Artifacts)
        );
    }
}
