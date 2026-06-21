/// Hard-delete and WAL checkpoint store for conversation retention.
///
/// Privacy invariant: deletion is permanent (hard delete, no soft-delete
/// tombstone). The WAL checkpoint after delete ensures disk space is reclaimed
/// and no unmanaged WAL state accumulates (adversarial hardening spec D-14).
///
/// The `messages` table carries `ON DELETE CASCADE` from `conversations`,
/// so deleting a conversation row removes all associated messages and
/// automatically keeps `messages_fts` in sync via the `messages_ad` trigger.
use crate::storage::sqlite::{SqlitePool, StorageError};

/// Typed store for conversation hard-delete and WAL checkpoint.
///
/// Wraps `SqlitePool` with a domain-specific delete API. IPC handlers must
/// not call `with_conn` directly — deletion must go through this store so the
/// WAL checkpoint invariant is always satisfied.
pub struct RetentionStore {
    pool: std::sync::Arc<SqlitePool>,
}

impl RetentionStore {
    /// Create a new `RetentionStore` sharing the given connection pool.
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Hard-delete a conversation and all its cascaded messages.
    ///
    /// Runs `PRAGMA wal_checkpoint(TRUNCATE)` after the delete to reclaim WAL
    /// disk space. If the checkpoint fails (e.g. busy readers), the error is
    /// logged as a non-fatal warning and does not prevent the delete response
    /// from completing (D-14).
    ///
    /// The `messages` table's `ON DELETE CASCADE` ensures all child messages
    /// are removed. The `messages_ad` FTS5 trigger keeps the FTS index in sync.
    ///
    /// If `id` does not exist the operation is a no-op and returns `Ok(())`.
    pub fn delete_conversation(&self, id: &str) -> Result<(), StorageError> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "DELETE FROM conversations WHERE id = ?1",
                rusqlite::params![id],
            )?;

            // WAL checkpoint — non-fatal if busy readers are present (D-14).
            if let Err(e) = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
                eprintln!("[retention] WAL checkpoint non-fatal warning: {e}");
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;
    use rusqlite::Connection;

    fn in_memory_pool() -> std::sync::Arc<SqlitePool> {
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
    fn delete_conversation_removes_conversation_and_messages() {
        let pool = in_memory_pool();
        let store = RetentionStore::new(pool.clone());

        // Seed data.
        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES ('conv-del', 'Delete Me')",
                [],
            )?;
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content)
                 VALUES ('msg-del-1', 'conv-del', 'user', 'first message'),
                        ('msg-del-2', 'conv-del', 'assistant', 'reply')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify pre-condition.
        let msg_count_before: i64 = pool
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM messages WHERE conversation_id = 'conv-del'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(msg_count_before, 2);

        // Delete.
        store.delete_conversation("conv-del").unwrap();

        // Conversation should be gone.
        let conv_count: i64 = pool
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM conversations WHERE id = 'conv-del'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(conv_count, 0, "conversation should be hard-deleted");

        // Messages should be cascade-deleted.
        let msg_count_after: i64 = pool
            .with_conn(|conn| {
                Ok(conn.query_row(
                    "SELECT COUNT(*) FROM messages WHERE conversation_id = 'conv-del'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(msg_count_after, 0, "messages should be cascade-deleted");
    }

    #[test]
    fn delete_nonexistent_conversation_is_noop() {
        let pool = in_memory_pool();
        let store = RetentionStore::new(pool);

        // Should not error when the row is absent.
        let result = store.delete_conversation("does-not-exist");
        assert!(result.is_ok(), "deleting absent conversation should be Ok");
    }
}
