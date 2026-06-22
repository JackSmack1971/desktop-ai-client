/// FTS5 full-text search store for conversation message content.
///
/// Privacy invariant: message content never crosses IPC directly — this store
/// is called only from typed IPC command handlers in `ipc::history`. The
/// frontend never receives raw FTS5 results; results are mapped to
/// `SearchResult` response types with controlled fields.
///
/// Security invariant: all FTS5 MATCH queries use SQLite bind parameters (?1).
/// User query strings are never interpolated into SQL text (T-03-04).
use crate::storage::sqlite::{SqlitePool, StorageError};

/// A single search result returned by `FtsStore::search`.
///
/// Carries conversation-level metadata plus a highlighted snippet from the
/// matching message content. The snippet is backend-generated from the user's
/// own stored content — no external data (T-03-02).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    /// Conversation primary key.
    pub id: String,
    /// Auto-generated conversation title (first user message, truncated).
    pub title: String,
    /// Model that completed this conversation (empty string if still active).
    pub model: String,
    /// Conversation status: `"active"`, `"complete"`, or `"incomplete"`.
    pub status: String,
    /// ISO datetime string of the last conversation update.
    pub updated_at: String,
    /// ~80-char highlighted snippet with `<b>` / `</b>` markers around matches.
    pub snippet: String,
}

/// Typed store for FTS5 full-text search over message content.
///
/// Wraps `SqlitePool` with a domain-specific search API so IPC handlers never
/// issue raw SQL. All FTS5 DDL lives in `migrations.rs` (migration 0003);
/// this store only issues read queries against the `messages_fts` virtual table.
pub struct FtsStore {
    pool: std::sync::Arc<SqlitePool>,
}

/// Escape arbitrary user input into a single FTS5 quoted phrase.
fn escape_fts5_phrase(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

impl FtsStore {
    /// Create a new `FtsStore` sharing the given connection pool.
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Search message content using an FTS5 MATCH query.
    ///
    /// Returns up to 50 conversations whose messages match `query`, ordered by
    /// FTS5 rank (best match first). Each result includes a short highlighted
    /// snippet (`<b>term</b>`) from the matching message.
    ///
    /// Returns an empty `Vec` when no messages match — `QueryReturnedNoRows`
    /// is treated as success, not an error.
    ///
    /// The `query` string is bound via SQLite parameters — it is never
    /// interpolated into the SQL text (T-03-04).
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, StorageError> {
        let escaped_query = escape_fts5_phrase(query);
        self.pool.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT c.id, c.title, c.model, c.status, c.updated_at,
                        (
                            SELECT snippet(messages_fts, 0, '<b>', '</b>', '\u{2026}', 15)
                            FROM messages_fts
                            WHERE messages_fts.conversation_id = c.id
                              AND messages_fts MATCH ?1
                            LIMIT 1
                        ) AS snippet
                 FROM conversations c
                 WHERE EXISTS (
                     SELECT 1
                     FROM messages_fts
                     WHERE messages_fts.conversation_id = c.id
                       AND messages_fts MATCH ?1
                 )
                 ORDER BY c.updated_at DESC
                 LIMIT 50",
            )?;

            let rows = stmt.query_map(rusqlite::params![escaped_query], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    model: row.get(2)?,
                    status: row.get(3)?,
                    updated_at: row.get(4)?,
                    snippet: row.get(5)?,
                })
            })?;

            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
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
    fn fts_search_returns_result_with_snippet() {
        let pool = in_memory_pool();
        let store = FtsStore::new(pool.clone());

        // Insert a conversation and message via the pool directly.
        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES ('conv-fts', 'FTS Test Conv')",
                [],
            )?;
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content)
                 VALUES ('msg-fts', 'conv-fts', 'user', 'hello rusqlite full text search')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let results = store.search("rusqlite").unwrap();
        assert!(
            !results.is_empty(),
            "search should return at least one result"
        );
        let r = &results[0];
        assert_eq!(r.id, "conv-fts");
        assert_eq!(r.title, "FTS Test Conv");
        assert!(!r.snippet.is_empty(), "snippet should not be empty");
    }

    #[test]
    fn fts_search_returns_empty_vec_on_no_match() {
        let pool = in_memory_pool();
        let store = FtsStore::new(pool.clone());

        // Insert a conversation with content that does not match the query.
        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES ('conv-nomatch', 'No Match')",
                [],
            )?;
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content)
                 VALUES ('msg-nomatch', 'conv-nomatch', 'user', 'unrelated content')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let results = store.search("xyzzy_not_in_db").unwrap();
        assert!(
            results.is_empty(),
            "search for absent term should return empty vec"
        );
    }

    #[test]
    fn fts_search_does_not_error_on_unbalanced_quote() {
        let pool = in_memory_pool();
        let store = FtsStore::new(pool.clone());

        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES ('conv-q', 'Quote Test')",
                [],
            )?;
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content)
                 VALUES ('msg-q', 'conv-q', 'user', 'some normal message text')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let result = store.search("\"unterminated");
        assert!(
            result.is_ok(),
            "search with an unbalanced quote should not error: {result:?}"
        );
    }

    #[test]
    fn fts_search_treats_operator_keywords_as_literal_text() {
        let pool = in_memory_pool();
        let store = FtsStore::new(pool.clone());

        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO conversations (id, title) VALUES ('conv-op', 'Operator Test')",
                [],
            )?;
            conn.execute(
                "INSERT INTO messages (id, conversation_id, role, content)
                 VALUES ('msg-op', 'conv-op', 'user', 'find the AND keyword in this sentence')",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let result = store.search("AND");
        assert!(
            result.is_ok(),
            "search for a bare boolean-operator keyword should not error: {result:?}"
        );
    }
}
