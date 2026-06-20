/// Conversation Transaction Protocol storage.
///
/// A `turn` is one user message and its eventual assistant response,
/// identified uniquely per conversation by a client-supplied
/// `idempotency_key` (`UNIQUE (conversation_id, idempotency_key)` in
/// migration 0005). A `turn_attempt` is one execution try at producing that
/// response — retries after `failed`/`failed_partial`/`cancelled` create a
/// new attempt under the same turn rather than a new turn, so the user
/// message is written exactly once no matter how many times the turn is
/// retried.
///
/// Every state-changing method here goes through `SqlitePool::with_transaction`
/// so the terminal write (assistant message + attempt status + turn status +
/// conversation status + artifact) lands atomically, and every terminal write
/// guards on `WHERE status = 'in_progress'` so a turn attempt resolves to
/// exactly one terminal state even if called twice (e.g. a duplicate
/// cancellation racing the stream's own terminal event).
use crate::storage::artifacts::{db_content_type, ArtifactContentType};
use crate::storage::sqlite::SqlitePool;
use rusqlite::params;
use std::sync::Arc;
use uuid::Uuid;

/// A new artifact to persist atomically alongside a successful attempt.
pub struct NewArtifact {
    pub id: String,
    pub content_type: ArtifactContentType,
    pub raw_source: String,
}

/// Outcome of `begin_turn` — what the caller should do next.
#[derive(Debug, Clone, PartialEq)]
pub enum BeginTurnOutcome {
    /// No turn existed for this `(conversation_id, idempotency_key)` pair.
    /// The caller should call `start_attempt(turn_id, 1)` next.
    New { turn_id: String },
    /// A turn exists and its most recent attempt ended in a non-terminal-success
    /// state (`failed_partial`, `failed`, or `cancelled`). The caller should
    /// call `start_attempt(turn_id, next_attempt_number)` to retry — the
    /// original user message is reused, never re-inserted.
    Retry {
        turn_id: String,
        next_attempt_number: i64,
    },
    /// A turn exists with an attempt still `in_progress`. The caller must not
    /// start a second concurrent attempt for the same turn.
    InFlight { turn_id: String },
    /// The turn already completed successfully. The caller should replay the
    /// cached result rather than calling the provider again.
    AlreadyComplete {
        turn_id: String,
        assistant_content: Option<String>,
        model: String,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
    },
}

/// Typed store for `turns` and `turn_attempts`. All reads and writes to those
/// tables must go through this store.
pub struct TurnStore {
    pool: Arc<SqlitePool>,
}

impl TurnStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Look up the turn for `(conversation_id, idempotency_key)`, or create
    /// one — inserting `new_message_content` as the turn's user message —
    /// when none exists yet.
    ///
    /// Idempotent: calling this twice with the same key never inserts a
    /// second user message or a second turn row. The lookup and the
    /// conditional insert happen inside one transaction over the single
    /// Mutex-guarded connection, so two concurrent calls with the same key
    /// cannot race past each other into a double insert.
    pub fn begin_turn(
        &self,
        conversation_id: &str,
        idempotency_key: &str,
        new_message_content: &str,
    ) -> rusqlite::Result<BeginTurnOutcome> {
        use rusqlite::OptionalExtension;

        self.pool.with_transaction(|tx| {
            let existing: Option<(String, String)> = tx
                .query_row(
                    "SELECT id, status FROM turns WHERE conversation_id = ?1 AND idempotency_key = ?2",
                    params![conversation_id, idempotency_key],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;

            let Some((existing_turn_id, status)) = existing else {
                let user_message_id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO messages (id, conversation_id, role, content, status)
                     VALUES (?1, ?2, 'user', ?3, 'complete')",
                    params![user_message_id, conversation_id, new_message_content],
                )?;
                let turn_id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO turns (id, conversation_id, user_message_id, idempotency_key)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![turn_id, conversation_id, user_message_id, idempotency_key],
                )?;
                return Ok(BeginTurnOutcome::New { turn_id });
            };

            match status.as_str() {
                "pending" => Ok(BeginTurnOutcome::InFlight {
                    turn_id: existing_turn_id,
                }),
                "complete" => {
                    let (model, prompt_tokens, completion_tokens, assistant_content): (
                        String,
                        Option<i64>,
                        Option<i64>,
                        Option<String>,
                    ) = tx.query_row(
                        "SELECT ta.model_id, ta.prompt_tokens, ta.completion_tokens, m.content
                         FROM turn_attempts ta
                         LEFT JOIN messages m ON m.id = ta.assistant_message_id
                         WHERE ta.turn_id = ?1 AND ta.status = 'complete'
                         ORDER BY ta.attempt_number DESC
                         LIMIT 1",
                        params![existing_turn_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )?;
                    Ok(BeginTurnOutcome::AlreadyComplete {
                        turn_id: existing_turn_id,
                        assistant_content,
                        model,
                        prompt_tokens: prompt_tokens.map(|v| v as u32),
                        completion_tokens: completion_tokens.map(|v| v as u32),
                    })
                }
                _ => {
                    // failed_partial / failed / cancelled — eligible for retry.
                    let next_attempt_number: i64 = tx.query_row(
                        "SELECT COALESCE(MAX(attempt_number), 0) + 1
                         FROM turn_attempts WHERE turn_id = ?1",
                        params![existing_turn_id],
                        |row| row.get(0),
                    )?;
                    Ok(BeginTurnOutcome::Retry {
                        turn_id: existing_turn_id,
                        next_attempt_number,
                    })
                }
            }
        })
    }

    /// Start a new attempt for `turn_id`, flipping the turn back to `pending`
    /// and inserting the `turn_attempts` row in one transaction.
    pub fn start_attempt(
        &self,
        turn_id: &str,
        attempt_number: i64,
    ) -> rusqlite::Result<String> {
        self.pool.with_transaction(|tx| {
            let attempt_id = Uuid::new_v4().to_string();
            tx.execute(
                "UPDATE turns SET status = 'pending', updated_at = datetime('now') WHERE id = ?1",
                params![turn_id],
            )?;
            tx.execute(
                "INSERT INTO turn_attempts (id, turn_id, attempt_number) VALUES (?1, ?2, ?3)",
                params![attempt_id, turn_id, attempt_number],
            )?;
            Ok(attempt_id)
        })
    }

    /// Atomically persist a successful attempt: assistant message, usage,
    /// resolved model, turn/conversation status, and an optional artifact.
    ///
    /// Returns `false` without writing anything if `attempt_id` was already
    /// terminal (the exactly-one-terminal-state guard) — callers should treat
    /// that as "someone else already resolved this attempt" and not retry.
    #[allow(clippy::too_many_arguments)]
    pub fn complete_attempt_success(
        &self,
        turn_id: &str,
        attempt_id: &str,
        conversation_id: &str,
        assistant_message_id: &str,
        content: &str,
        model: &str,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
        artifact: Option<NewArtifact>,
    ) -> rusqlite::Result<bool> {
        self.pool.with_transaction(|tx| {
            let updated = tx.execute(
                "UPDATE turn_attempts
                 SET status = 'complete', model_id = ?2, prompt_tokens = ?3, completion_tokens = ?4,
                     assistant_message_id = ?5, updated_at = datetime('now')
                 WHERE id = ?1 AND status = 'in_progress'",
                params![
                    attempt_id,
                    model,
                    prompt_tokens.map(|v| v as i64),
                    completion_tokens.map(|v| v as i64),
                    assistant_message_id,
                ],
            )?;
            if updated == 0 {
                return Ok(false);
            }

            tx.execute(
                "INSERT INTO messages (id, conversation_id, role, content, status, turn_id)
                 VALUES (?1, ?2, 'assistant', ?3, 'complete', ?4)",
                params![assistant_message_id, conversation_id, content, turn_id],
            )?;
            tx.execute(
                "UPDATE turns SET status = 'complete', updated_at = datetime('now') WHERE id = ?1",
                params![turn_id],
            )?;
            tx.execute(
                "UPDATE conversations SET status = 'complete', model = ?2, updated_at = datetime('now')
                 WHERE id = ?1",
                params![conversation_id, model],
            )?;

            if let Some(artifact) = artifact {
                let (content_type_value, language_value) = db_content_type(&artifact.content_type);
                tx.execute(
                    "INSERT INTO artifacts (id, conversation_id, message_id, content_type, language, raw_source)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        artifact.id,
                        conversation_id,
                        assistant_message_id,
                        content_type_value,
                        language_value,
                        artifact.raw_source,
                    ],
                )?;
            }

            Ok(true)
        })
    }

    /// Atomically persist a partial-failure attempt (truncated stream, or a
    /// mid-stream provider error after some content had already streamed).
    /// Always inserts the partial assistant message — even if empty — so the
    /// turn's terminal state is auditable.
    pub fn complete_attempt_failed_partial(
        &self,
        turn_id: &str,
        attempt_id: &str,
        conversation_id: &str,
        assistant_message_id: &str,
        partial_content: &str,
        reason: &str,
    ) -> rusqlite::Result<bool> {
        self.pool.with_transaction(|tx| {
            let updated = tx.execute(
                "UPDATE turn_attempts
                 SET status = 'failed_partial', failure_reason = ?2, assistant_message_id = ?3,
                     updated_at = datetime('now')
                 WHERE id = ?1 AND status = 'in_progress'",
                params![attempt_id, reason, assistant_message_id],
            )?;
            if updated == 0 {
                return Ok(false);
            }

            tx.execute(
                "INSERT INTO messages (id, conversation_id, role, content, status, turn_id)
                 VALUES (?1, ?2, 'assistant', ?3, 'incomplete', ?4)",
                params![assistant_message_id, conversation_id, partial_content, turn_id],
            )?;
            tx.execute(
                "UPDATE turns SET status = 'failed_partial', updated_at = datetime('now') WHERE id = ?1",
                params![turn_id],
            )?;
            tx.execute(
                "UPDATE conversations SET status = 'incomplete', updated_at = datetime('now') WHERE id = ?1",
                params![conversation_id],
            )?;
            Ok(true)
        })
    }

    /// Atomically persist a user-cancelled attempt, preserving whatever
    /// partial text had streamed before cancellation.
    pub fn complete_attempt_cancelled(
        &self,
        turn_id: &str,
        attempt_id: &str,
        conversation_id: &str,
        assistant_message_id: &str,
        partial_content: &str,
    ) -> rusqlite::Result<bool> {
        self.pool.with_transaction(|tx| {
            let updated = tx.execute(
                "UPDATE turn_attempts
                 SET status = 'cancelled', failure_reason = 'frontend_cancelled', assistant_message_id = ?2,
                     updated_at = datetime('now')
                 WHERE id = ?1 AND status = 'in_progress'",
                params![attempt_id, assistant_message_id],
            )?;
            if updated == 0 {
                return Ok(false);
            }

            tx.execute(
                "INSERT INTO messages (id, conversation_id, role, content, status, turn_id)
                 VALUES (?1, ?2, 'assistant', ?3, 'incomplete', ?4)",
                params![assistant_message_id, conversation_id, partial_content, turn_id],
            )?;
            tx.execute(
                "UPDATE turns SET status = 'cancelled', updated_at = datetime('now') WHERE id = ?1",
                params![turn_id],
            )?;
            tx.execute(
                "UPDATE conversations SET status = 'incomplete', updated_at = datetime('now') WHERE id = ?1",
                params![conversation_id],
            )?;
            Ok(true)
        })
    }

    /// Atomically persist a hard failure with no usable partial output (e.g.
    /// the provider connection never produced a single byte). No assistant
    /// message is written — there is nothing to preserve.
    pub fn complete_attempt_failed(
        &self,
        turn_id: &str,
        attempt_id: &str,
        conversation_id: &str,
        reason: &str,
    ) -> rusqlite::Result<bool> {
        self.pool.with_transaction(|tx| {
            let updated = tx.execute(
                "UPDATE turn_attempts
                 SET status = 'failed', failure_reason = ?2, updated_at = datetime('now')
                 WHERE id = ?1 AND status = 'in_progress'",
                params![attempt_id, reason],
            )?;
            if updated == 0 {
                return Ok(false);
            }

            tx.execute(
                "UPDATE turns SET status = 'failed', updated_at = datetime('now') WHERE id = ?1",
                params![turn_id],
            )?;
            tx.execute(
                "UPDATE conversations SET status = 'incomplete', updated_at = datetime('now') WHERE id = ?1",
                params![conversation_id],
            )?;
            Ok(true)
        })
    }

    /// Recovery: on startup, any attempt still `in_progress` belonged to a
    /// process that no longer exists (crash or forced quit) — its partial
    /// text lived only in memory and is unrecoverable. Flip those attempts
    /// (and their parent turns/conversations) to a terminal `failed` state
    /// with reason `backend_shutdown` so the UI never shows a stream
    /// spinning forever. Returns the number of attempts recovered.
    pub fn recover_orphaned_attempts(&self) -> rusqlite::Result<usize> {
        self.pool.with_transaction(|tx| {
            let recovered = tx.execute(
                "UPDATE turn_attempts SET status = 'failed', failure_reason = 'backend_shutdown',
                    updated_at = datetime('now')
                 WHERE status = 'in_progress'",
                [],
            )?;
            tx.execute(
                "UPDATE turns SET status = 'failed', updated_at = datetime('now')
                 WHERE status = 'pending'
                   AND id IN (SELECT turn_id FROM turn_attempts WHERE failure_reason = 'backend_shutdown')",
                [],
            )?;
            tx.execute(
                "UPDATE conversations SET status = 'incomplete', updated_at = datetime('now')
                 WHERE status = 'active'
                   AND id IN (
                     SELECT t.conversation_id FROM turns t
                     JOIN turn_attempts ta ON ta.turn_id = t.id
                     WHERE ta.failure_reason = 'backend_shutdown'
                   )",
                [],
            )?;
            Ok(recovered)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;
    use crate::storage::sqlite::{ConversationStore, MessageStore};
    use rusqlite::Connection;

    fn migrated_pool() -> Arc<SqlitePool> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        run_migrations(&conn, "0.0.0-test").unwrap();
        Arc::new(SqlitePool::from_connection(conn))
    }

    /// Seed an empty conversation, returning its id. The user message is
    /// created by `begin_turn` itself, not seeded ahead of time.
    fn seed_conversation(pool: &Arc<SqlitePool>) -> String {
        let conv_store = ConversationStore::new(pool.clone());
        let conversation_id = Uuid::new_v4().to_string();
        conv_store
            .create_conversation(&conversation_id, "Test")
            .unwrap();
        conversation_id
    }

    #[test]
    fn begin_turn_is_new_for_first_submission() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);

        let outcome = store
            .begin_turn(&conversation_id, "idem-1", "hello")
            .unwrap();
        assert!(matches!(outcome, BeginTurnOutcome::New { .. }));
    }

    #[test]
    fn begin_turn_is_in_flight_while_attempt_pending() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);

        let BeginTurnOutcome::New { turn_id } = store
            .begin_turn(&conversation_id, "idem-1", "hello")
            .unwrap()
        else {
            panic!("expected New");
        };
        store.start_attempt(&turn_id, 1).unwrap();

        let second = store.begin_turn(&conversation_id, "idem-1", "hello").unwrap();
        assert_eq!(second, BeginTurnOutcome::InFlight { turn_id });
    }

    #[test]
    fn begin_turn_does_not_duplicate_user_message_across_retries() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);
        let msg_store = MessageStore::new(pool.clone());

        let BeginTurnOutcome::New { turn_id } = store
            .begin_turn(&conversation_id, "idem-retry", "hello")
            .unwrap()
        else {
            panic!("expected New");
        };
        let attempt_1 = store.start_attempt(&turn_id, 1).unwrap();
        store
            .complete_attempt_failed(&turn_id, &attempt_1, &conversation_id, "provider_connection_lost")
            .unwrap();

        // Retry with the same idempotency_key — must reuse the turn, not
        // create a second user message.
        let outcome = store
            .begin_turn(&conversation_id, "idem-retry", "hello")
            .unwrap();
        assert_eq!(
            outcome,
            BeginTurnOutcome::Retry {
                turn_id: turn_id.clone(),
                next_attempt_number: 2,
            }
        );

        let messages = msg_store.get_messages(&conversation_id).unwrap();
        let user_messages: Vec<_> = messages.iter().filter(|m| m.role == "user").collect();
        assert_eq!(
            user_messages.len(),
            1,
            "retry must not insert a second user message row"
        );
    }

    #[test]
    fn begin_turn_returns_already_complete_for_finished_turn() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);

        let BeginTurnOutcome::New { turn_id } = store
            .begin_turn(&conversation_id, "idem-done", "hello")
            .unwrap()
        else {
            panic!("expected New");
        };
        let attempt_id = store.start_attempt(&turn_id, 1).unwrap();
        store
            .complete_attempt_success(
                &turn_id,
                &attempt_id,
                &conversation_id,
                &Uuid::new_v4().to_string(),
                "the answer",
                "test-model",
                Some(10),
                Some(20),
                None,
            )
            .unwrap();

        let outcome = store
            .begin_turn(&conversation_id, "idem-done", "hello")
            .unwrap();
        match outcome {
            BeginTurnOutcome::AlreadyComplete {
                assistant_content,
                model,
                prompt_tokens,
                completion_tokens,
                ..
            } => {
                assert_eq!(assistant_content, Some("the answer".to_string()));
                assert_eq!(model, "test-model");
                assert_eq!(prompt_tokens, Some(10));
                assert_eq!(completion_tokens, Some(20));
            }
            other => panic!("expected AlreadyComplete, got {other:?}"),
        }
    }

    #[test]
    fn complete_attempt_success_is_atomic_and_terminal_once() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);
        let conv_store = ConversationStore::new(pool.clone());
        let msg_store = MessageStore::new(pool.clone());

        let BeginTurnOutcome::New { turn_id } = store
            .begin_turn(&conversation_id, "idem-success", "hello")
            .unwrap()
        else {
            panic!("expected New");
        };
        let attempt_id = store.start_attempt(&turn_id, 1).unwrap();
        let assistant_message_id = Uuid::new_v4().to_string();

        let wrote = store
            .complete_attempt_success(
                &turn_id,
                &attempt_id,
                &conversation_id,
                &assistant_message_id,
                "hello back",
                "test-model",
                Some(5),
                Some(7),
                None,
            )
            .unwrap();
        assert!(wrote);

        let conv = conv_store.get_conversation(&conversation_id).unwrap().unwrap();
        assert_eq!(conv.status, "complete");
        assert_eq!(conv.model, "test-model");
        let messages = msg_store.get_messages(&conversation_id).unwrap();
        assert!(messages.iter().any(|m| m.id == assistant_message_id));

        // Exactly-one-terminal-state guard: a second terminal write for the
        // same attempt must be a no-op, not a second insert.
        let wrote_again = store
            .complete_attempt_success(
                &turn_id,
                &attempt_id,
                &conversation_id,
                &assistant_message_id,
                "hello back AGAIN",
                "test-model",
                None,
                None,
                None,
            )
            .unwrap();
        assert!(!wrote_again, "terminal write must not apply twice");
        let messages_after = msg_store.get_messages(&conversation_id).unwrap();
        assert_eq!(
            messages_after.len(),
            messages.len(),
            "double terminal write must not duplicate the assistant message"
        );
    }

    #[test]
    fn complete_attempt_failed_partial_persists_partial_text() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);
        let conv_store = ConversationStore::new(pool.clone());

        let BeginTurnOutcome::New { turn_id } = store
            .begin_turn(&conversation_id, "idem-truncated", "hello")
            .unwrap()
        else {
            panic!("expected New");
        };
        let attempt_id = store.start_attempt(&turn_id, 1).unwrap();
        let assistant_message_id = Uuid::new_v4().to_string();

        store
            .complete_attempt_failed_partial(
                &turn_id,
                &attempt_id,
                &conversation_id,
                &assistant_message_id,
                "partial tex",
                "provider_connection_lost",
            )
            .unwrap();

        let conv = conv_store.get_conversation(&conversation_id).unwrap().unwrap();
        assert_eq!(conv.status, "incomplete");
    }

    #[test]
    fn complete_attempt_failed_writes_no_assistant_message() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);
        let msg_store = MessageStore::new(pool.clone());

        let BeginTurnOutcome::New { turn_id } = store
            .begin_turn(&conversation_id, "idem-failed", "hello")
            .unwrap()
        else {
            panic!("expected New");
        };
        let attempt_id = store.start_attempt(&turn_id, 1).unwrap();

        store
            .complete_attempt_failed(&turn_id, &attempt_id, &conversation_id, "provider_connection_lost")
            .unwrap();

        let messages = msg_store.get_messages(&conversation_id).unwrap();
        assert_eq!(messages.len(), 1, "only the original user message should exist");
    }

    #[test]
    fn recover_orphaned_attempts_flips_in_progress_to_failed() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);
        let conv_store = ConversationStore::new(pool.clone());

        let BeginTurnOutcome::New { turn_id } = store
            .begin_turn(&conversation_id, "idem-crash", "hello")
            .unwrap()
        else {
            panic!("expected New");
        };
        store.start_attempt(&turn_id, 1).unwrap();
        // Simulate a crash: the attempt is left `in_progress` forever.

        let recovered = store.recover_orphaned_attempts().unwrap();
        assert_eq!(recovered, 1);

        let conv = conv_store.get_conversation(&conversation_id).unwrap().unwrap();
        assert_eq!(conv.status, "incomplete");

        // Recovery must be idempotent — running it again recovers nothing new.
        let recovered_again = store.recover_orphaned_attempts().unwrap();
        assert_eq!(recovered_again, 0);
    }

    /// Three sequential turns in the same conversation — the scenario behind
    /// "hydrate history, append only the new turn". Each turn must persist
    /// exactly one user + one assistant message, in order, under a distinct
    /// `turn_id`, with no duplication of earlier turns' content.
    #[test]
    fn three_turn_continuity_in_one_conversation() {
        let pool = migrated_pool();
        let store = TurnStore::new(pool.clone());
        let conversation_id = seed_conversation(&pool);
        let msg_store = MessageStore::new(pool.clone());

        let mut turn_ids = Vec::new();
        for (idx, (user_text, assistant_text)) in [
            ("first question", "first answer"),
            ("second question", "second answer"),
            ("third question", "third answer"),
        ]
        .iter()
        .enumerate()
        {
            let idempotency_key = format!("idem-turn-{idx}");
            let BeginTurnOutcome::New { turn_id } = store
                .begin_turn(&conversation_id, &idempotency_key, user_text)
                .unwrap()
            else {
                panic!("expected New for turn {idx}");
            };
            assert!(
                !turn_ids.contains(&turn_id),
                "each turn must get a distinct turn_id"
            );
            let attempt_id = store.start_attempt(&turn_id, 1).unwrap();
            let wrote = store
                .complete_attempt_success(
                    &turn_id,
                    &attempt_id,
                    &conversation_id,
                    &Uuid::new_v4().to_string(),
                    assistant_text,
                    "test-model",
                    Some(1),
                    Some(1),
                    None,
                )
                .unwrap();
            assert!(wrote);
            turn_ids.push(turn_id);
        }

        let messages = msg_store.get_messages(&conversation_id).unwrap();
        assert_eq!(messages.len(), 6, "3 turns × (user + assistant) = 6 messages");
        let roles: Vec<&str> = messages.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(
            roles,
            vec!["user", "assistant", "user", "assistant", "user", "assistant"]
        );
        let contents: Vec<&str> = messages.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(
            contents,
            vec![
                "first question",
                "first answer",
                "second question",
                "second answer",
                "third question",
                "third answer",
            ],
            "hydrating this conversation later must replay turns in order with no duplication"
        );
        assert_eq!(turn_ids.len(), 3);
    }
}
