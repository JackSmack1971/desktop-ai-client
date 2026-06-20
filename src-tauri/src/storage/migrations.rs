/// Database migration engine.
///
/// Migrations run inside transactions where SQLite permits. Each migration
/// records its result in `schema_migrations` so the runner can detect
/// partial or failed runs and enter recovery mode.
///
/// Ordering invariant: migrations must be listed in strictly ascending order
/// by `id`. Re-running an already-applied migration is a no-op (idempotent).
///
/// Privacy invariant: do not log or return migration SQL that contains user
/// content, secrets, or local file paths.
use rusqlite::{params, Connection};

/// A single database migration.
pub struct Migration {
    /// Unique, monotonically increasing migration identifier (e.g. "0001").
    pub id: &'static str,
    /// A short human-readable description for diagnostics.
    pub description: &'static str,
    /// SQL to run for this migration. Must be safe to wrap in a transaction.
    pub sql: &'static str,
}

/// All migrations in ascending order.
///
/// Each migration must have a unique `id`. Adding a migration appends to this
/// list; never reorder or modify an existing entry after it ships.
///
/// Note: the `schema_migrations` tracking table itself is created by the
/// inline bootstrap in `run_migrations` before this slice is iterated.
/// It is intentionally absent from this list to avoid a misleading
/// "applied 2 migrations" count on a fresh database.
pub static MIGRATIONS: &[Migration] = &[
    Migration {
        id: "0001",
        description: "Create shell_preferences table for backend-owned surface state",
        sql: "
            CREATE TABLE IF NOT EXISTS shell_preferences (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              active_surface TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
        ",
    },
    Migration {
        id: "0002",
        description: "Create conversations and messages tables",
        sql: "
            CREATE TABLE IF NOT EXISTS conversations (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL,
              model TEXT NOT NULL DEFAULT '',
              status TEXT NOT NULL DEFAULT 'active'
                CHECK (status IN ('active', 'complete', 'incomplete')),
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE TABLE IF NOT EXISTS messages (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL
                REFERENCES conversations(id) ON DELETE CASCADE,
              role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
              content TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'complete'
                CHECK (status IN ('complete', 'incomplete')),
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_messages_conversation_id
              ON messages(conversation_id);
        ",
    },
    Migration {
        id: "0003",
        description: "Create FTS5 external-content table and sync triggers for messages",
        sql: "
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
              content,
              conversation_id UNINDEXED,
              content='messages',
              content_rowid='rowid',
              tokenize='unicode61'
            );

            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
              INSERT INTO messages_fts(rowid, content, conversation_id)
                VALUES (new.rowid, new.content, new.conversation_id);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
              INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id)
                VALUES ('delete', old.rowid, old.content, old.conversation_id);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
              INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id)
                VALUES ('delete', old.rowid, old.content, old.conversation_id);
              INSERT INTO messages_fts(rowid, content, conversation_id)
                VALUES (new.rowid, new.content, new.conversation_id);
            END;
        ",
    },
    Migration {
        id: "0004",
        description: "Create artifacts table for sanitized preview storage",
        sql: "
            CREATE TABLE IF NOT EXISTS artifacts (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL
                REFERENCES conversations(id) ON DELETE CASCADE,
              message_id TEXT
                REFERENCES messages(id) ON DELETE SET NULL,
              content_type TEXT NOT NULL
                CHECK (content_type IN ('html', 'svg', 'plain_text', 'code')),
              language TEXT NOT NULL DEFAULT '',
              raw_source TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_artifacts_conversation_id
              ON artifacts(conversation_id);

            CREATE INDEX IF NOT EXISTS idx_artifacts_message_id
              ON artifacts(message_id);
        ",
    },
    Migration {
        id: "0005",
        description: "Create turns/turn_attempts tables for the conversation transaction protocol",
        sql: "
            CREATE TABLE IF NOT EXISTS turns (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL
                REFERENCES conversations(id) ON DELETE CASCADE,
              user_message_id TEXT NOT NULL
                REFERENCES messages(id) ON DELETE CASCADE,
              idempotency_key TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'complete', 'failed_partial', 'cancelled', 'failed')),
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              UNIQUE (conversation_id, idempotency_key)
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_turns_conversation_id
              ON turns(conversation_id);

            CREATE TABLE IF NOT EXISTS turn_attempts (
              id TEXT PRIMARY KEY,
              turn_id TEXT NOT NULL
                REFERENCES turns(id) ON DELETE CASCADE,
              attempt_number INTEGER NOT NULL,
              assistant_message_id TEXT
                REFERENCES messages(id) ON DELETE SET NULL,
              provider_id TEXT NOT NULL DEFAULT 'openrouter',
              model_id TEXT NOT NULL DEFAULT '',
              status TEXT NOT NULL DEFAULT 'in_progress'
                CHECK (status IN ('in_progress', 'complete', 'failed_partial', 'cancelled', 'failed')),
              failure_reason TEXT,
              prompt_tokens INTEGER,
              completion_tokens INTEGER,
              last_sequence INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now')),
              UNIQUE (turn_id, attempt_number)
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_turn_attempts_turn_id
              ON turn_attempts(turn_id);

            ALTER TABLE messages ADD COLUMN turn_id TEXT REFERENCES turns(id);
        ",
    },
    Migration {
        id: "0006",
        description: "Create Evidence-Gated Memory Engine tables (Phase 1, shadow mode)",
        sql: "
            -- Immutable run traces: one per turn outcome. Insert-only — the
            -- BEFORE UPDATE trigger below blocks edits. DELETE is still
            -- permitted so conversation retention (hard delete) can cascade;
            -- see docs/memory-loop.md for the rollback/retention rationale.
            CREATE TABLE IF NOT EXISTS memory_run_traces (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL
                REFERENCES conversations(id) ON DELETE CASCADE,
              turn_id TEXT REFERENCES turns(id),
              task_summary TEXT NOT NULL,
              outcome TEXT NOT NULL
                CHECK (outcome IN ('success', 'failed_partial', 'failed', 'cancelled')),
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_memory_run_traces_conversation_id
              ON memory_run_traces(conversation_id);

            CREATE TRIGGER IF NOT EXISTS memory_run_traces_immutable
            BEFORE UPDATE ON memory_run_traces
            BEGIN
              SELECT RAISE(ABORT, 'memory_run_traces rows are immutable; insert a new trace instead');
            END;

            -- Candidate memories. Every proposal is inserted (including
            -- duplicates and contradictions, which land with status
            -- 'rejected') so the decision ledger stays inspectable.
            CREATE TABLE IF NOT EXISTS memory_candidates (
              id TEXT PRIMARY KEY,
              kind TEXT NOT NULL
                CHECK (kind IN ('factual', 'episodic', 'procedural', 'caution')),
              summary TEXT NOT NULL,
              dedup_key TEXT NOT NULL,
              source_run_trace_id TEXT NOT NULL
                REFERENCES memory_run_traces(id) ON DELETE CASCADE,
              confidence REAL NOT NULL DEFAULT 0.5
                CHECK (confidence >= 0.0 AND confidence <= 1.0),
              utility INTEGER NOT NULL DEFAULT 0,
              status TEXT NOT NULL DEFAULT 'candidate'
                CHECK (status IN ('candidate', 'promoted', 'rejected', 'expired')),
              verification_state TEXT NOT NULL DEFAULT 'unverified'
                CHECK (verification_state IN ('unverified', 'verified', 'refuted')),
              contradiction_state TEXT NOT NULL DEFAULT 'none'
                CHECK (contradiction_state IN ('none', 'contradicted')),
              contradicts_candidate_id TEXT
                REFERENCES memory_candidates(id) ON DELETE SET NULL,
              expires_at TEXT,
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_memory_candidates_dedup_key
              ON memory_candidates(dedup_key);
            CREATE INDEX IF NOT EXISTS idx_memory_candidates_status
              ON memory_candidates(status);
            CREATE INDEX IF NOT EXISTS idx_memory_candidates_source_run_trace_id
              ON memory_candidates(source_run_trace_id);

            -- The decision ledger: one immutable row per promotion/rejection
            -- decision made about a candidate. Append-only — see the
            -- BEFORE UPDATE trigger below.
            CREATE TABLE IF NOT EXISTS memory_decisions (
              id TEXT PRIMARY KEY,
              candidate_id TEXT NOT NULL
                REFERENCES memory_candidates(id) ON DELETE CASCADE,
              action TEXT NOT NULL
                CHECK (action IN ('proposed', 'promoted', 'rejected', 'deferred', 'expired')),
              reason TEXT NOT NULL,
              decided_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_memory_decisions_candidate_id
              ON memory_decisions(candidate_id);

            CREATE TRIGGER IF NOT EXISTS memory_decisions_immutable
            BEFORE UPDATE ON memory_decisions
            BEGIN
              SELECT RAISE(ABORT, 'memory_decisions rows are immutable; insert a new decision instead');
            END;

            -- Shadow-mode retrieval log: records what bounded retrieval would
            -- have returned. Never read by chat_send / providers::routing —
            -- this table exists purely so replay fixtures can measure
            -- precision/recall without memories influencing a live prompt.
            CREATE TABLE IF NOT EXISTS memory_retrieval_log (
              id TEXT PRIMARY KEY,
              kind_filter TEXT,
              returned_candidate_ids TEXT NOT NULL,
              requested_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;
        ",
    },
];

/// Apply any pending migrations to `conn`.
///
/// Returns the number of migrations applied. Logs the migration ID and
/// description at info level; does not log SQL or user data.
pub fn run_migrations(conn: &Connection, app_version: &str) -> rusqlite::Result<usize> {
    // Bootstrap the tracking table before iterating the MIGRATIONS slice.
    // This table is not listed in MIGRATIONS itself — doing so would produce
    // a misleading "applied 2" count on a fresh database and create a
    // confusing dual-creation path that complicates incident diagnosis.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           id TEXT PRIMARY KEY,
           description TEXT NOT NULL,
           app_version TEXT NOT NULL,
           applied_at TEXT NOT NULL,
           success INTEGER NOT NULL CHECK (success IN (0, 1))
         );",
    )?;

    let mut applied = 0;

    for migration in MIGRATIONS {
        // Skip migrations that already succeeded.
        let already_applied: bool = conn
            .query_row(
                "SELECT success FROM schema_migrations WHERE id = ?1",
                params![migration.id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);

        if already_applied {
            continue;
        }

        // Run the migration in a savepoint (nested transaction) so failures
        // can be rolled back without losing the tracking table state.

        // Validate id is a safe SQL identifier before embedding it.
        debug_assert!(
            migration
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "migration id must be alphanumeric/underscore only: {:?}",
            migration.id
        );
        // SAFETY: migration.id and migration.sql are &'static str literals defined
        // in MIGRATIONS. Do not generalize this pattern to dynamic values.
        let result = conn.execute_batch(&format!(
            "SAVEPOINT migration_{id};\n{sql}\nRELEASE SAVEPOINT migration_{id};",
            id = migration.id,
            sql = migration.sql,
        ));

        let success = result.is_ok();

        conn.execute(
            "INSERT INTO schema_migrations (id, description, app_version, applied_at, success)
             VALUES (?1, ?2, ?3, datetime('now'), ?4)
             ON CONFLICT(id) DO UPDATE SET success = ?4, applied_at = datetime('now')",
            params![
                migration.id,
                migration.description,
                app_version,
                success as i64,
            ],
        )?;

        result?;
        applied += 1;
    }

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    #[test]
    fn migrations_apply_to_fresh_database() {
        let conn = fresh_conn();
        let applied = run_migrations(&conn, "0.1.0").unwrap();
        // schema_migrations is bootstrapped outside the MIGRATIONS slice;
        // only the domain migrations (currently 4) are counted here.
        assert_eq!(
            applied,
            MIGRATIONS.len(),
            "all migrations should apply on a fresh db"
        );
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();
        let second_run = run_migrations(&conn, "0.1.0").unwrap();
        assert_eq!(second_run, 0, "re-running migrations should be a no-op");
    }

    #[test]
    fn shell_preferences_table_exists_after_migration() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        // Verify the table can be inserted into without error.
        conn.execute(
            "INSERT INTO shell_preferences (id, active_surface, updated_at)
             VALUES (1, 'chat', datetime('now'))",
            [],
        )
        .unwrap();

        let surface: String = conn
            .query_row(
                "SELECT active_surface FROM shell_preferences WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(surface, "chat");
    }

    #[test]
    fn migrations_count_is_six() {
        assert_eq!(
            MIGRATIONS.len(),
            6,
            "expected 6 migrations after the memory engine addition"
        );
    }

    #[test]
    fn memory_run_traces_table_exists_and_is_immutable() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-mem', 'Memory Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_run_traces (id, conversation_id, task_summary, outcome)
             VALUES ('trace-1', 'conv-mem', 'answered a question', 'success')",
            [],
        )
        .unwrap();

        let outcome: String = conn
            .query_row(
                "SELECT outcome FROM memory_run_traces WHERE id = 'trace-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(outcome, "success");

        // Immutability: UPDATE must be rejected by the BEFORE UPDATE trigger.
        let update = conn.execute(
            "UPDATE memory_run_traces SET outcome = 'failed' WHERE id = 'trace-1'",
            [],
        );
        assert!(update.is_err(), "run traces must be immutable");
    }

    #[test]
    fn memory_run_traces_cascade_deletes_with_conversation() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-mem-del', 'Memory Delete Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_run_traces (id, conversation_id, task_summary, outcome)
             VALUES ('trace-del', 'conv-mem-del', 'task', 'success')",
            [],
        )
        .unwrap();

        conn.execute("DELETE FROM conversations WHERE id = 'conv-mem-del'", [])
            .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_run_traces WHERE id = 'trace-del'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "run traces must cascade-delete with their conversation");
    }

    #[test]
    fn memory_candidates_and_decisions_tables_exist_after_migration() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-cand', 'Candidate Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_run_traces (id, conversation_id, task_summary, outcome)
             VALUES ('trace-cand', 'conv-cand', 'task', 'success')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_candidates (id, kind, summary, dedup_key, source_run_trace_id)
             VALUES ('cand-1', 'factual', 'the API rate limit is 60rpm', 'factual:the api rate limit is 60rpm', 'trace-cand')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO memory_decisions (id, candidate_id, action, reason)
             VALUES ('dec-1', 'cand-1', 'proposed', 'new_candidate')",
            [],
        )
        .unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM memory_candidates WHERE id = 'cand-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "candidate");

        // Ledger immutability.
        let update = conn.execute(
            "UPDATE memory_decisions SET reason = 'edited' WHERE id = 'dec-1'",
            [],
        );
        assert!(update.is_err(), "decision ledger rows must be immutable");
    }

    /// Rollback rehearsal: manually dropping the memory_* tables (the
    /// documented recovery path, since this migration runner has no
    /// down-migrations) must leave the core conversation transaction
    /// protocol tables untouched.
    #[test]
    fn dropping_memory_tables_does_not_affect_core_tables() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-rb', 'Rollback Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-rb', 'conv-rb', 'user', 'hello')",
            [],
        )
        .unwrap();

        conn.execute_batch(
            "DROP TABLE memory_retrieval_log;
             DROP TABLE memory_decisions;
             DROP TABLE memory_candidates;
             DROP TABLE memory_run_traces;",
        )
        .unwrap();

        let title: String = conn
            .query_row(
                "SELECT title FROM conversations WHERE id = 'conv-rb'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Rollback Test");
        let content: String = conn
            .query_row("SELECT content FROM messages WHERE id = 'msg-rb'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn conversations_and_messages_tables_exist_after_migration() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        // Insert a conversation row.
        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-1', 'Test Conversation')",
            [],
        )
        .unwrap();

        // Query it back.
        let title: String = conn
            .query_row(
                "SELECT title FROM conversations WHERE id = 'conv-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Test Conversation");

        // Insert a message row with FK to conversation.
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-1', 'conv-1', 'user', 'Hello world')",
            [],
        )
        .unwrap();

        let content: String = conn
            .query_row(
                "SELECT content FROM messages WHERE id = 'msg-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(content, "Hello world");
    }

    #[test]
    fn conversations_status_check_enforced() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        // Valid status should succeed.
        conn.execute(
            "INSERT INTO conversations (id, title, status) VALUES ('conv-ok', 'T', 'complete')",
            [],
        )
        .unwrap();

        // Invalid status should be rejected.
        let bad = conn.execute(
            "INSERT INTO conversations (id, title, status) VALUES ('conv-bad', 'T', 'invalid')",
            [],
        );
        assert!(
            bad.is_err(),
            "conversations.status CHECK constraint should reject 'invalid'"
        );
    }

    #[test]
    fn messages_cascade_deletes_with_conversation() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-cascade', 'Cascade Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-cascade', 'conv-cascade', 'user', 'cascade me')",
            [],
        )
        .unwrap();

        // Delete the conversation — messages should cascade.
        conn.execute("DELETE FROM conversations WHERE id = 'conv-cascade'", [])
            .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE conversation_id = 'conv-cascade'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "messages should be deleted via ON DELETE CASCADE");
    }

    #[test]
    fn fts5_table_and_triggers_exist() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        // Insert a conversation and message — the messages_ai trigger should fire.
        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-fts', 'FTS Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-fts', 'conv-fts', 'user', 'searchable text content')",
            [],
        )
        .unwrap();

        // Search the FTS5 table directly.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM messages_fts WHERE messages_fts MATCH 'searchable'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            count > 0,
            "FTS5 table should contain the inserted message content"
        );
    }

    #[test]
    fn artifacts_table_exists_after_migration() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-art', 'Artifact Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-art', 'conv-art', 'assistant', '<div>hi</div>')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, conversation_id, message_id, content_type, language, raw_source)
             VALUES ('art-1', 'conv-art', 'msg-art', 'html', '', '<div>hi</div>')",
            [],
        )
        .unwrap();

        let content_type: String = conn
            .query_row(
                "SELECT content_type FROM artifacts WHERE id = 'art-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(content_type, "html");
    }

    #[test]
    fn turns_and_turn_attempts_tables_exist_after_migration() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-turn', 'Turn Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-turn-user', 'conv-turn', 'user', 'hello')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO turns (id, conversation_id, user_message_id, idempotency_key)
             VALUES ('turn-1', 'conv-turn', 'msg-turn-user', 'idem-1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO turn_attempts (id, turn_id, attempt_number) VALUES ('attempt-1', 'turn-1', 1)",
            [],
        )
        .unwrap();

        let status: String = conn
            .query_row("SELECT status FROM turns WHERE id = 'turn-1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(status, "pending");

        // Duplicate idempotency_key for the same conversation must be rejected.
        let dup = conn.execute(
            "INSERT INTO turns (id, conversation_id, user_message_id, idempotency_key)
             VALUES ('turn-2', 'conv-turn', 'msg-turn-user', 'idem-1')",
            [],
        );
        assert!(
            dup.is_err(),
            "duplicate (conversation_id, idempotency_key) should violate UNIQUE constraint"
        );
    }

    #[test]
    fn messages_turn_id_column_exists_after_migration() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-mt', 'Msg Turn Test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-mt-1', 'conv-mt', 'user', 'hi')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO turns (id, conversation_id, user_message_id, idempotency_key)
             VALUES ('turn-mt', 'conv-mt', 'msg-mt-1', 'idem-mt')",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE messages SET turn_id = 'turn-mt' WHERE id = 'msg-mt-1'",
            [],
        )
        .unwrap();

        let turn_id: String = conn
            .query_row(
                "SELECT turn_id FROM messages WHERE id = 'msg-mt-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(turn_id, "turn-mt");
    }
}
