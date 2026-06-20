/// Evidence-Gated Memory Engine storage (Phase 1, shadow mode).
///
/// Shadow-mode invariant: nothing in this module is called from
/// `ipc::chat` or `providers::routing`. Candidates and decisions are
/// recorded for inspection and replay measurement only — they must not
/// influence a live prompt until a later phase explicitly wires retrieval
/// into `chat_send`.
///
/// Lifecycle: a `run_trace` (immutable) is recorded for a finished turn.
/// `propose_candidate` derives a candidate memory from that trace —
/// duplicates (by `dedup_key`) are inserted with `status = 'rejected'` so
/// every proposal stays inspectable in `memory_decisions`, the append-only
/// decision ledger. `decide_promotion` applies the deterministic promotion
/// policy from `docs/memory-loop.md` to move a `candidate` row to
/// `promoted`/`rejected`/`deferred`. `bounded_retrieve` only ever returns
/// `promoted`, non-expired rows, and itself only writes to
/// `memory_retrieval_log` — never to a chat request.
use crate::storage::sqlite::SqlitePool;
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind {
    Factual,
    Episodic,
    Procedural,
    Caution,
}

impl MemoryKind {
    fn as_str(self) -> &'static str {
        match self {
            MemoryKind::Factual => "factual",
            MemoryKind::Episodic => "episodic",
            MemoryKind::Procedural => "procedural",
            MemoryKind::Caution => "caution",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "factual" => MemoryKind::Factual,
            "episodic" => MemoryKind::Episodic,
            "procedural" => MemoryKind::Procedural,
            "caution" => MemoryKind::Caution,
            other => panic!("unknown memory kind in storage: {other:?}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationState {
    Unverified,
    Verified,
    Refuted,
}

impl VerificationState {
    fn as_str(self) -> &'static str {
        match self {
            VerificationState::Unverified => "unverified",
            VerificationState::Verified => "verified",
            VerificationState::Refuted => "refuted",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "unverified" => VerificationState::Unverified,
            "verified" => VerificationState::Verified,
            "refuted" => VerificationState::Refuted,
            other => panic!("unknown verification state in storage: {other:?}"),
        }
    }
}

/// A candidate memory row as read back from storage.
#[derive(Debug, Clone)]
pub struct CandidateRow {
    pub id: String,
    pub kind: MemoryKind,
    pub summary: String,
    pub source_run_trace_id: String,
    pub confidence: f64,
    pub utility: i64,
    pub status: String,
    pub verification_state: VerificationState,
    pub contradiction_state: String,
}

/// Outcome of proposing a candidate memory.
#[derive(Debug, Clone, PartialEq)]
pub enum ProposeOutcome {
    /// A new candidate row was inserted with `status = 'candidate'`.
    Proposed { candidate_id: String },
    /// An active (non-rejected) candidate with the same `dedup_key` already
    /// existed. The new row was still inserted, but with `status =
    /// 'rejected'`, so the proposal stays visible in the ledger.
    Duplicate {
        candidate_id: String,
        duplicate_of: String,
    },
}

/// Outcome of a promotion decision.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Promoted,
    Rejected(&'static str),
    /// Not promoted yet, but not rejected either — the candidate's
    /// `status` stays `'candidate'` (not terminal) so it can be
    /// re-evaluated later once more evidence (e.g. `record_reuse`) accrues.
    Deferred(&'static str),
    /// The candidate was not in `status = 'candidate'` (e.g. already
    /// promoted/rejected) — no decision was made.
    Skipped,
}

/// Aggregate counts for `memory-health` diagnostics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MemoryHealth {
    pub candidate_count: i64,
    pub promoted_count: i64,
    pub rejected_count: i64,
    pub deferred_count: i64,
    pub expired_count: i64,
    pub contradicted_count: i64,
    pub run_trace_count: i64,
    pub decision_count: i64,
}

/// Deterministic promotion policy (`docs/memory-loop.md`).
///
/// ponytail: the "judge" here is this mechanical rule, not an LLM judge —
/// Phase 1 has no live judge call. Upgrade path: replace the body with a
/// call to an actual judge once Phase 2 wires retrieval into a live prompt.
fn promotion_rule(
    kind: MemoryKind,
    confidence: f64,
    utility: i64,
    verification_state: VerificationState,
    contradiction_state: &str,
) -> Decision {
    if contradiction_state == "contradicted" {
        return Decision::Rejected("contradiction_detected");
    }

    match kind {
        MemoryKind::Factual => match verification_state {
            VerificationState::Refuted => Decision::Rejected("verification_refuted"),
            VerificationState::Verified => Decision::Promoted,
            VerificationState::Unverified if confidence >= 0.85 => Decision::Promoted,
            VerificationState::Unverified => Decision::Deferred("unverified_below_confidence_floor"),
        },
        MemoryKind::Episodic => {
            if utility >= 2 || confidence >= 0.9 {
                Decision::Promoted
            } else {
                Decision::Deferred("insufficient_reuse_signal")
            }
        }
        MemoryKind::Procedural => {
            if confidence >= 0.7 && utility >= 1 {
                Decision::Promoted
            } else {
                Decision::Deferred("not_yet_proven_in_use")
            }
        }
        MemoryKind::Caution => {
            if confidence >= 0.4 {
                Decision::Promoted
            } else {
                Decision::Rejected("confidence_too_low_to_retain")
            }
        }
    }
}

/// Normalize a summary into a duplicate-detection key: kind plus
/// lowercased, whitespace-collapsed summary text. Exact-match only —
/// fuzzy/semantic dedup is out of scope for Phase 1.
fn dedup_key(kind: MemoryKind, summary: &str) -> String {
    let normalized: String = summary.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    format!("{}:{normalized}", kind.as_str())
}

/// Typed store for the memory engine's `memory_run_traces`,
/// `memory_candidates`, `memory_decisions`, and `memory_retrieval_log`
/// tables. All reads and writes to those tables must go through this store.
pub struct MemoryStore {
    pool: Arc<SqlitePool>,
}

impl MemoryStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Record an immutable run trace for a finished turn. Returns the new
    /// trace id.
    pub fn record_run_trace(
        &self,
        conversation_id: &str,
        turn_id: Option<&str>,
        task_summary: &str,
        outcome: &str,
    ) -> rusqlite::Result<String> {
        let id = Uuid::new_v4().to_string();
        self.pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO memory_run_traces (id, conversation_id, turn_id, task_summary, outcome)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, conversation_id, turn_id, task_summary, outcome],
            )?;
            Ok(())
        })?;
        Ok(id)
    }

    /// Propose a candidate memory derived from `source_run_trace_id`.
    /// Always inserts a row; duplicates are inserted pre-rejected (see
    /// `ProposeOutcome::Duplicate`). Every proposal — accepted or not — gets
    /// a `memory_decisions` ledger entry.
    pub fn propose_candidate(
        &self,
        kind: MemoryKind,
        summary: &str,
        source_run_trace_id: &str,
        confidence: f64,
    ) -> rusqlite::Result<ProposeOutcome> {
        let key = dedup_key(kind, summary);
        self.pool.with_transaction(|tx| {
            let existing: Option<String> = tx
                .query_row(
                    "SELECT id FROM memory_candidates
                     WHERE dedup_key = ?1 AND status != 'rejected'
                     ORDER BY created_at ASC LIMIT 1",
                    params![key],
                    |row| row.get(0),
                )
                .optional()?;

            let candidate_id = Uuid::new_v4().to_string();
            let decision_id = Uuid::new_v4().to_string();

            if let Some(duplicate_of) = existing {
                tx.execute(
                    "INSERT INTO memory_candidates
                     (id, kind, summary, dedup_key, source_run_trace_id, confidence, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'rejected')",
                    params![candidate_id, kind.as_str(), summary, key, source_run_trace_id, confidence],
                )?;
                tx.execute(
                    "INSERT INTO memory_decisions (id, candidate_id, action, reason)
                     VALUES (?1, ?2, 'rejected', ?3)",
                    params![decision_id, candidate_id, format!("duplicate_of:{duplicate_of}")],
                )?;
                Ok(ProposeOutcome::Duplicate {
                    candidate_id,
                    duplicate_of,
                })
            } else {
                tx.execute(
                    "INSERT INTO memory_candidates
                     (id, kind, summary, dedup_key, source_run_trace_id, confidence)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![candidate_id, kind.as_str(), summary, key, source_run_trace_id, confidence],
                )?;
                tx.execute(
                    "INSERT INTO memory_decisions (id, candidate_id, action, reason)
                     VALUES (?1, ?2, 'proposed', 'new_candidate')",
                    params![decision_id, candidate_id],
                )?;
                Ok(ProposeOutcome::Proposed { candidate_id })
            }
        })
    }

    /// Apply the deterministic promotion policy to a candidate currently in
    /// `status = 'candidate'`. No-ops (returns `Decision::Skipped`) for a
    /// candidate already decided (e.g. rejected at proposal time).
    pub fn decide_promotion(&self, candidate_id: &str) -> rusqlite::Result<Decision> {
        self.pool.with_transaction(|tx| {
            let row: Option<(String, f64, i64, String, String, String)> = tx
                .query_row(
                    "SELECT kind, confidence, utility, status, verification_state, contradiction_state
                     FROM memory_candidates WHERE id = ?1",
                    params![candidate_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    },
                )
                .optional()?;

            let Some((kind, confidence, utility, status, verification_state, contradiction_state)) = row
            else {
                return Ok(Decision::Skipped);
            };
            if status != "candidate" {
                return Ok(Decision::Skipped);
            }

            let decision = promotion_rule(
                MemoryKind::parse(&kind),
                confidence,
                utility,
                VerificationState::parse(&verification_state),
                &contradiction_state,
            );

            let (new_status, action, reason): (&str, &str, &'static str) = match decision {
                Decision::Promoted => ("promoted", "promoted", "promotion_rule_satisfied"),
                Decision::Rejected(reason) => ("rejected", "rejected", reason),
                Decision::Deferred(reason) => ("candidate", "deferred", reason),
                Decision::Skipped => unreachable!("promotion_rule never returns Skipped"),
            };

            tx.execute(
                "UPDATE memory_candidates SET status = ?2, updated_at = datetime('now') WHERE id = ?1",
                params![candidate_id, new_status],
            )?;
            tx.execute(
                "INSERT INTO memory_decisions (id, candidate_id, action, reason)
                 VALUES (?1, ?2, ?3, ?4)",
                params![Uuid::new_v4().to_string(), candidate_id, action, reason],
            )?;

            Ok(decision)
        })
    }

    /// Mark two candidates as mutually contradicting. Phase 1 resolves a
    /// contradiction by rejecting both sides rather than attempting
    /// automatic reconciliation.
    /// ponytail: no reconciliation flow yet — add one when a human/judge
    /// review step exists to pick a winner instead of rejecting both.
    pub fn mark_contradiction(&self, candidate_a: &str, candidate_b: &str) -> rusqlite::Result<()> {
        self.pool.with_transaction(|tx| {
            for (this, other) in [(candidate_a, candidate_b), (candidate_b, candidate_a)] {
                tx.execute(
                    "UPDATE memory_candidates
                     SET contradiction_state = 'contradicted', contradicts_candidate_id = ?2,
                         status = 'rejected', updated_at = datetime('now')
                     WHERE id = ?1",
                    params![this, other],
                )?;
                tx.execute(
                    "INSERT INTO memory_decisions (id, candidate_id, action, reason)
                     VALUES (?1, ?2, 'rejected', ?3)",
                    params![
                        Uuid::new_v4().to_string(),
                        this,
                        format!("contradicts:{other}")
                    ],
                )?;
            }
            Ok(())
        })
    }

    /// Record an external verification outcome for a candidate (e.g. a
    /// fact checked against a trusted source). Does not itself write to the
    /// decision ledger — only `decide_promotion`'s promote/reject/defer
    /// transitions are ledgered; this just supplies the evidence that
    /// `promotion_rule` reads for `MemoryKind::Factual`.
    pub fn set_verification_state(
        &self,
        candidate_id: &str,
        state: VerificationState,
    ) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "UPDATE memory_candidates SET verification_state = ?2, updated_at = datetime('now') WHERE id = ?1",
                params![candidate_id, state.as_str()],
            )?;
            Ok(())
        })
    }

    /// Record that a promoted candidate was usefully reused (a replay
    /// fixture hit, or — once a later phase wires it in — a live retrieval
    /// that helped). Increments `utility` by 1.
    pub fn record_reuse(&self, candidate_id: &str) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "UPDATE memory_candidates SET utility = utility + 1, updated_at = datetime('now') WHERE id = ?1",
                params![candidate_id],
            )?;
            Ok(())
        })
    }

    /// Sweep candidates past `expires_at` into `status = 'expired'`. Returns
    /// the number of rows expired.
    pub fn expire_stale(&self) -> rusqlite::Result<usize> {
        self.pool.with_transaction(|tx| {
            let mut stmt = tx.prepare(
                "SELECT id FROM memory_candidates
                 WHERE status IN ('candidate', 'promoted')
                   AND expires_at IS NOT NULL AND expires_at <= datetime('now')",
            )?;
            let ids: Vec<String> = stmt
                .query_map([], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            drop(stmt);

            for id in &ids {
                tx.execute(
                    "UPDATE memory_candidates SET status = 'expired', updated_at = datetime('now') WHERE id = ?1",
                    params![id],
                )?;
                tx.execute(
                    "INSERT INTO memory_decisions (id, candidate_id, action, reason)
                     VALUES (?1, ?2, 'expired', 'past_expires_at')",
                    params![Uuid::new_v4().to_string(), id],
                )?;
            }
            Ok(ids.len())
        })
    }

    /// Bounded retrieval: only ever returns `promoted`, non-expired
    /// candidates, ranked by confidence then recency, capped at `limit`.
    /// Logs the query to `memory_retrieval_log` for replay measurement.
    ///
    /// Shadow-mode invariant: this method is exercised only by tests and
    /// `telemetry::memory_replay` in Phase 1 — `ipc::chat` must not call it.
    pub fn bounded_retrieve(
        &self,
        kind_filter: Option<MemoryKind>,
        limit: usize,
    ) -> rusqlite::Result<Vec<CandidateRow>> {
        self.pool.with_transaction(|tx| {
            let rows: Vec<CandidateRow> = match kind_filter {
                Some(kind) => {
                    let mut stmt = tx.prepare(
                        "SELECT id, kind, summary, source_run_trace_id, confidence, utility, status, verification_state, contradiction_state
                         FROM memory_candidates
                         WHERE status = 'promoted' AND kind = ?1
                           AND (expires_at IS NULL OR expires_at > datetime('now'))
                         ORDER BY confidence DESC, created_at DESC LIMIT ?2",
                    )?;
                    let collected: Vec<CandidateRow> = stmt
                        .query_map(params![kind.as_str(), limit as i64], row_to_candidate)?
                        .collect::<rusqlite::Result<_>>()?;
                    collected
                }
                None => {
                    let mut stmt = tx.prepare(
                        "SELECT id, kind, summary, source_run_trace_id, confidence, utility, status, verification_state, contradiction_state
                         FROM memory_candidates
                         WHERE status = 'promoted'
                           AND (expires_at IS NULL OR expires_at > datetime('now'))
                         ORDER BY confidence DESC, created_at DESC LIMIT ?1",
                    )?;
                    let collected: Vec<CandidateRow> = stmt
                        .query_map(params![limit as i64], row_to_candidate)?
                        .collect::<rusqlite::Result<_>>()?;
                    collected
                }
            };

            let ids_json = format!(
                "[{}]",
                rows.iter().map(|r| format!("\"{}\"", r.id)).collect::<Vec<_>>().join(",")
            );
            tx.execute(
                "INSERT INTO memory_retrieval_log (id, kind_filter, returned_candidate_ids)
                 VALUES (?1, ?2, ?3)",
                params![
                    Uuid::new_v4().to_string(),
                    kind_filter.map(|k| k.as_str()),
                    ids_json
                ],
            )?;

            Ok(rows)
        })
    }

    /// Memory-health diagnostics: aggregate counts across all memory engine
    /// tables, for inspection (e.g. a future `memory_health` IPC command or
    /// CLI diagnostic — not wired up in Phase 1).
    pub fn memory_health(&self) -> rusqlite::Result<MemoryHealth> {
        self.pool.with_conn(|conn| {
            let mut health = MemoryHealth::default();
            health.run_trace_count = conn.query_row("SELECT COUNT(*) FROM memory_run_traces", [], |r| r.get(0))?;
            health.decision_count = conn.query_row("SELECT COUNT(*) FROM memory_decisions", [], |r| r.get(0))?;
            health.candidate_count = conn.query_row(
                "SELECT COUNT(*) FROM memory_candidates WHERE status = 'candidate'",
                [],
                |r| r.get(0),
            )?;
            health.promoted_count = conn.query_row(
                "SELECT COUNT(*) FROM memory_candidates WHERE status = 'promoted'",
                [],
                |r| r.get(0),
            )?;
            health.rejected_count = conn.query_row(
                "SELECT COUNT(*) FROM memory_candidates WHERE status = 'rejected'",
                [],
                |r| r.get(0),
            )?;
            health.expired_count = conn.query_row(
                "SELECT COUNT(*) FROM memory_candidates WHERE status = 'expired'",
                [],
                |r| r.get(0),
            )?;
            health.contradicted_count = conn.query_row(
                "SELECT COUNT(*) FROM memory_candidates WHERE contradiction_state = 'contradicted'",
                [],
                |r| r.get(0),
            )?;
            // deferred candidates stay in status='candidate'; the ledger is
            // the source of truth for how many *deferral decisions* fired.
            health.deferred_count = conn.query_row(
                "SELECT COUNT(*) FROM memory_decisions WHERE action = 'deferred'",
                [],
                |r| r.get(0),
            )?;
            Ok(health)
        })
    }
}

fn row_to_candidate(row: &rusqlite::Row) -> rusqlite::Result<CandidateRow> {
    let kind: String = row.get(1)?;
    let verification_state: String = row.get(7)?;
    Ok(CandidateRow {
        id: row.get(0)?,
        kind: MemoryKind::parse(&kind),
        summary: row.get(2)?,
        source_run_trace_id: row.get(3)?,
        confidence: row.get(4)?,
        utility: row.get(5)?,
        status: row.get(6)?,
        verification_state: VerificationState::parse(&verification_state),
        contradiction_state: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;
    use crate::storage::sqlite::ConversationStore;
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

    fn seed_trace(pool: &Arc<SqlitePool>, store: &MemoryStore) -> String {
        let conversation_id = Uuid::new_v4().to_string();
        ConversationStore::new(pool.clone())
            .create_conversation(&conversation_id, "Memory Test")
            .unwrap();
        store
            .record_run_trace(&conversation_id, None, "answered a question", "success")
            .unwrap()
    }

    #[test]
    fn propose_candidate_inserts_new_row_with_candidate_status() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let outcome = store
            .propose_candidate(MemoryKind::Factual, "the API rate limit is 60rpm", &trace_id, 0.9)
            .unwrap();
        assert!(matches!(outcome, ProposeOutcome::Proposed { .. }));
    }

    #[test]
    fn propose_candidate_detects_exact_duplicate() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let first = store
            .propose_candidate(MemoryKind::Factual, "the API rate limit is 60rpm", &trace_id, 0.9)
            .unwrap();
        let ProposeOutcome::Proposed { candidate_id: first_id } = first else {
            panic!("expected Proposed");
        };

        // Same kind + summary, different whitespace/case — still a duplicate.
        let second = store
            .propose_candidate(MemoryKind::Factual, "  THE api RATE limit IS   60rpm  ", &trace_id, 0.5)
            .unwrap();
        match second {
            ProposeOutcome::Duplicate { duplicate_of, .. } => assert_eq!(duplicate_of, first_id),
            other => panic!("expected Duplicate, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_proposal_is_inserted_pre_rejected_and_ledgered() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        store
            .propose_candidate(MemoryKind::Caution, "watch out for X", &trace_id, 0.5)
            .unwrap();
        let ProposeOutcome::Duplicate { candidate_id, .. } = store
            .propose_candidate(MemoryKind::Caution, "watch out for X", &trace_id, 0.5)
            .unwrap()
        else {
            panic!("expected Duplicate");
        };

        let status: String = pool
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT status FROM memory_candidates WHERE id = ?1",
                    params![candidate_id],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert_eq!(status, "rejected");

        let reason: String = pool
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT reason FROM memory_decisions WHERE candidate_id = ?1",
                    params![candidate_id],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert!(reason.starts_with("duplicate_of:"));
    }

    #[test]
    fn decide_promotion_promotes_high_confidence_factual() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id } = store
            .propose_candidate(MemoryKind::Factual, "fact A", &trace_id, 0.9)
            .unwrap()
        else {
            panic!("expected Proposed");
        };

        let decision = store.decide_promotion(&candidate_id).unwrap();
        assert_eq!(decision, Decision::Promoted);
    }

    #[test]
    fn decide_promotion_promotes_low_confidence_factual_once_verified() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id } = store
            .propose_candidate(MemoryKind::Factual, "fact, externally checked", &trace_id, 0.3)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        // Below the unverified confidence floor, but external verification
        // should make it promotable regardless of confidence.
        store
            .set_verification_state(&candidate_id, VerificationState::Verified)
            .unwrap();

        assert_eq!(store.decide_promotion(&candidate_id).unwrap(), Decision::Promoted);
    }

    #[test]
    fn decide_promotion_rejects_refuted_factual_even_at_high_confidence() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id } = store
            .propose_candidate(MemoryKind::Factual, "fact, later debunked", &trace_id, 0.95)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        store
            .set_verification_state(&candidate_id, VerificationState::Refuted)
            .unwrap();

        assert_eq!(
            store.decide_promotion(&candidate_id).unwrap(),
            Decision::Rejected("verification_refuted")
        );
    }

    #[test]
    fn decide_promotion_defers_low_confidence_unverified_factual() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id } = store
            .propose_candidate(MemoryKind::Factual, "fact B", &trace_id, 0.3)
            .unwrap()
        else {
            panic!("expected Proposed");
        };

        let decision = store.decide_promotion(&candidate_id).unwrap();
        assert_eq!(decision, Decision::Deferred("unverified_below_confidence_floor"));
    }

    #[test]
    fn decide_promotion_episodic_requires_reuse_or_high_confidence() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id } = store
            .propose_candidate(MemoryKind::Episodic, "ran into flaky test", &trace_id, 0.6)
            .unwrap()
        else {
            panic!("expected Proposed");
        };

        assert_eq!(
            store.decide_promotion(&candidate_id).unwrap(),
            Decision::Deferred("insufficient_reuse_signal")
        );

        // A deferred candidate stays in status = 'candidate' (not a
        // terminal state) so it can be re-evaluated once reuse accumulates.
        store.record_reuse(&candidate_id).unwrap();
        store.record_reuse(&candidate_id).unwrap();
        assert_eq!(store.decide_promotion(&candidate_id).unwrap(), Decision::Promoted);
    }

    #[test]
    fn decide_promotion_is_skipped_for_already_rejected_duplicate() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        store
            .propose_candidate(MemoryKind::Caution, "dup", &trace_id, 0.9)
            .unwrap();
        let ProposeOutcome::Duplicate { candidate_id, .. } = store
            .propose_candidate(MemoryKind::Caution, "dup", &trace_id, 0.9)
            .unwrap()
        else {
            panic!("expected Duplicate");
        };

        assert_eq!(store.decide_promotion(&candidate_id).unwrap(), Decision::Skipped);
    }

    #[test]
    fn mark_contradiction_rejects_both_candidates() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id: a } = store
            .propose_candidate(MemoryKind::Factual, "the limit is 60rpm", &trace_id, 0.9)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        let ProposeOutcome::Proposed { candidate_id: b } = store
            .propose_candidate(MemoryKind::Factual, "the limit is 120rpm", &trace_id, 0.9)
            .unwrap()
        else {
            panic!("expected Proposed");
        };

        store.mark_contradiction(&a, &b).unwrap();

        for id in [&a, &b] {
            let (status, contradiction_state): (String, String) = pool
                .with_conn(|conn| {
                    conn.query_row(
                        "SELECT status, contradiction_state FROM memory_candidates WHERE id = ?1",
                        params![id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                })
                .unwrap();
            assert_eq!(status, "rejected");
            assert_eq!(contradiction_state, "contradicted");
        }

        // A contradicted candidate must never be promotable, even if it
        // somehow returns to 'candidate' status — defense in depth via the
        // promotion_rule's own contradiction_state check is covered by
        // decide_promotion_is_skipped tests above (status != 'candidate'
        // already blocks it here).
    }

    #[test]
    fn expire_stale_moves_past_expiry_candidates_to_expired() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id } = store
            .propose_candidate(MemoryKind::Episodic, "stale fact", &trace_id, 0.9)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        pool.with_conn(|conn| {
            conn.execute(
                "UPDATE memory_candidates SET expires_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
                params![candidate_id],
            )
        })
        .unwrap();

        let expired = store.expire_stale().unwrap();
        assert_eq!(expired, 1);

        let status: String = pool
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT status FROM memory_candidates WHERE id = ?1",
                    params![candidate_id],
                    |row| row.get(0),
                )
            })
            .unwrap();
        assert_eq!(status, "expired");
    }

    #[test]
    fn bounded_retrieve_only_returns_promoted_non_expired_candidates() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id: promoted_id } = store
            .propose_candidate(MemoryKind::Factual, "promoted fact", &trace_id, 0.95)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        store.decide_promotion(&promoted_id).unwrap();

        let ProposeOutcome::Proposed { candidate_id: deferred_id } = store
            .propose_candidate(MemoryKind::Factual, "deferred fact", &trace_id, 0.1)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        store.decide_promotion(&deferred_id).unwrap();

        let results = store.bounded_retrieve(None, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, promoted_id);
    }

    #[test]
    fn bounded_retrieve_respects_limit_and_logs_query() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        for i in 0..3 {
            let ProposeOutcome::Proposed { candidate_id } = store
                .propose_candidate(MemoryKind::Caution, &format!("caution {i}"), &trace_id, 0.9)
                .unwrap()
            else {
                panic!("expected Proposed");
            };
            store.decide_promotion(&candidate_id).unwrap();
        }

        let results = store.bounded_retrieve(Some(MemoryKind::Caution), 2).unwrap();
        assert_eq!(results.len(), 2, "bounded retrieval must respect the limit");

        let log_count: i64 = pool
            .with_conn(|conn| conn.query_row("SELECT COUNT(*) FROM memory_retrieval_log", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(log_count, 1, "bounded_retrieve must log its query exactly once");
    }

    #[test]
    fn memory_health_reports_accurate_counts() {
        let pool = migrated_pool();
        let store = MemoryStore::new(pool.clone());
        let trace_id = seed_trace(&pool, &store);

        let ProposeOutcome::Proposed { candidate_id: promoted_id } = store
            .propose_candidate(MemoryKind::Factual, "fact one", &trace_id, 0.95)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        store.decide_promotion(&promoted_id).unwrap();

        let ProposeOutcome::Proposed { candidate_id: deferred_id } = store
            .propose_candidate(MemoryKind::Procedural, "method one", &trace_id, 0.2)
            .unwrap()
        else {
            panic!("expected Proposed");
        };
        store.decide_promotion(&deferred_id).unwrap();

        store
            .propose_candidate(MemoryKind::Caution, "watch out", &trace_id, 0.9)
            .unwrap();
        store
            .propose_candidate(MemoryKind::Caution, "watch out", &trace_id, 0.9)
            .unwrap(); // duplicate -> rejected

        let health = store.memory_health().unwrap();
        assert_eq!(health.run_trace_count, 1);
        assert_eq!(health.promoted_count, 1);
        assert_eq!(health.rejected_count, 1, "the duplicate caution candidate");
        assert_eq!(health.deferred_count, 1, "one deferred decision was logged");
        // candidate_count: the deferred procedural (stays 'candidate' for
        // re-evaluation) plus the proposed-but-undecided caution.
        assert_eq!(health.candidate_count, 2);
    }
}
