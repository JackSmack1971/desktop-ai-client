/// Deterministic replay harness for the Evidence-Gated Memory Engine
/// (Phase 1, shadow mode).
///
/// Each fixture case seeds its own fresh in-memory database (no shared
/// state, no wall-clock dependency in the computed metrics) so the report
/// is reproducible run over run. This never touches a live provider or
/// chat request — it only exercises `storage::memory::MemoryStore`
/// directly, the same shadow-mode boundary the store itself documents.
use crate::storage::memory::{MemoryKind, MemoryStore, ProposeOutcome};
use crate::storage::migrations::run_migrations;
use crate::storage::sqlite::{ConversationStore, SqlitePool};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct FixtureCandidate {
    pub name: &'static str,
    pub kind: MemoryKind,
    pub summary: &'static str,
    pub confidence: f64,
    /// Number of `record_reuse` calls applied before the promotion decision
    /// — simulates "this episodic memory was reused N times".
    pub reuse_bumps: i64,
}

pub struct FixtureCase {
    pub name: &'static str,
    pub candidates: Vec<FixtureCandidate>,
    /// Names of two candidates in this case to mark as contradicting each
    /// other before promotion decisions run.
    pub contradicting_pair: Option<(&'static str, &'static str)>,
    pub query_kind: MemoryKind,
    /// Candidate names considered ground-truth relevant for this case's
    /// retrieval query.
    pub relevant_names: Vec<&'static str>,
    /// Whether the task succeeds with no memory available at all.
    pub baseline_success: bool,
    /// Whether the task succeeds when a relevant memory was retrieved.
    pub success_if_relevant_retrieved: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplayReport {
    pub cases_run: usize,
    /// Of everything bounded_retrieve returned, the fraction that was
    /// ground-truth relevant (micro-averaged across all cases).
    pub precision: f64,
    /// Of everything ground-truth relevant, the fraction bounded_retrieve
    /// actually surfaced (micro-averaged across all cases).
    pub useful_recall: f64,
    /// Contradicted candidates / all candidates proposed across the fixture.
    pub contradiction_rate: f64,
    /// Estimated token cost (chars / 4) of everything actually retrieved.
    pub token_cost_total: usize,
    /// avg(task success with memory) - avg(task success without memory).
    pub task_delta: f64,
}

/// The frozen Phase 1 fixture set. Adding a case changes the frozen
/// expected metrics in this module's test — update both together.
pub fn default_fixture() -> Vec<FixtureCase> {
    vec![
        FixtureCase {
            name: "duplicate_detection",
            candidates: vec![
                FixtureCandidate {
                    name: "fact_a",
                    kind: MemoryKind::Factual,
                    summary: "the rate limit is sixty",
                    confidence: 0.9,
                    reuse_bumps: 0,
                },
                FixtureCandidate {
                    name: "fact_a_dup",
                    kind: MemoryKind::Factual,
                    summary: "THE RATE LIMIT IS SIXTY",
                    confidence: 0.9,
                    reuse_bumps: 0,
                },
            ],
            contradicting_pair: None,
            query_kind: MemoryKind::Factual,
            relevant_names: vec!["fact_a"],
            baseline_success: false,
            success_if_relevant_retrieved: true,
        },
        FixtureCase {
            name: "contradiction_suppresses_both",
            candidates: vec![
                FixtureCandidate {
                    name: "fact_b",
                    kind: MemoryKind::Factual,
                    summary: "the limit is sixty per minute",
                    confidence: 0.9,
                    reuse_bumps: 0,
                },
                FixtureCandidate {
                    name: "fact_c",
                    kind: MemoryKind::Factual,
                    summary: "the limit is one twenty per minute",
                    confidence: 0.9,
                    reuse_bumps: 0,
                },
            ],
            contradicting_pair: Some(("fact_b", "fact_c")),
            query_kind: MemoryKind::Factual,
            relevant_names: vec![],
            baseline_success: true,
            success_if_relevant_retrieved: false,
        },
        FixtureCase {
            name: "episodic_reuse_promotion",
            candidates: vec![FixtureCandidate {
                name: "ep_a",
                kind: MemoryKind::Episodic,
                summary: "watch the retry loop",
                confidence: 0.5,
                reuse_bumps: 2,
            }],
            contradicting_pair: None,
            query_kind: MemoryKind::Episodic,
            relevant_names: vec!["ep_a"],
            baseline_success: false,
            success_if_relevant_retrieved: true,
        },
        FixtureCase {
            name: "caution_low_confidence_rejected",
            candidates: vec![FixtureCandidate {
                name: "caution_a",
                kind: MemoryKind::Caution,
                summary: "skip retries on 429",
                confidence: 0.2,
                reuse_bumps: 0,
            }],
            contradicting_pair: None,
            query_kind: MemoryKind::Caution,
            relevant_names: vec![],
            baseline_success: true,
            success_if_relevant_retrieved: false,
        },
    ]
}

fn fresh_store() -> (Arc<SqlitePool>, MemoryStore) {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
        .expect("set pragmas");
    run_migrations(&conn, "0.0.0-replay").expect("run migrations");
    let pool = Arc::new(SqlitePool::from_connection(conn));
    let store = MemoryStore::new(pool.clone());
    (pool, store)
}

/// Run every case in `fixture` against its own fresh in-memory database and
/// micro-average the resulting metrics. Deterministic given a fixed fixture.
pub fn run_replay(fixture: &[FixtureCase]) -> ReplayReport {
    let mut total_retrieved = 0usize;
    let mut total_relevant_retrieved = 0usize;
    let mut total_relevant_expected = 0usize;
    let mut total_contradicted = 0i64;
    let mut total_candidates = 0i64;
    let mut token_cost_total = 0usize;
    let mut with_memory_successes = 0.0f64;
    let mut baseline_successes = 0.0f64;

    for case in fixture {
        let (pool, store) = fresh_store();
        let conversation_id = format!("replay-{}", case.name);
        ConversationStore::new(pool.clone())
            .create_conversation(&conversation_id, case.name)
            .expect("seed conversation");
        let trace_id = store
            .record_run_trace(&conversation_id, None, case.name, "success")
            .expect("record run trace");

        let mut name_to_id: HashMap<&'static str, String> = HashMap::new();
        for candidate in &case.candidates {
            let outcome = store
                .propose_candidate(candidate.kind, candidate.summary, &trace_id, candidate.confidence)
                .expect("propose candidate");
            let id = match outcome {
                ProposeOutcome::Proposed { candidate_id } => candidate_id,
                ProposeOutcome::Duplicate { candidate_id, .. } => candidate_id,
            };
            for _ in 0..candidate.reuse_bumps {
                store.record_reuse(&id).expect("record reuse");
            }
            name_to_id.insert(candidate.name, id);
        }

        if let Some((a, b)) = case.contradicting_pair {
            store
                .mark_contradiction(&name_to_id[a], &name_to_id[b])
                .expect("mark contradiction");
        }

        for id in name_to_id.values() {
            let _ = store.decide_promotion(id).expect("decide promotion");
        }

        let retrieved = store
            .bounded_retrieve(Some(case.query_kind), 10)
            .expect("bounded retrieve");
        let id_to_name: HashMap<&str, &'static str> =
            name_to_id.iter().map(|(name, id)| (id.as_str(), *name)).collect();
        let retrieved_names: Vec<&'static str> = retrieved
            .iter()
            .map(|row| id_to_name[row.id.as_str()])
            .collect();

        let relevant_set: HashSet<&'static str> = case.relevant_names.iter().copied().collect();
        let relevant_retrieved = retrieved_names.iter().filter(|n| relevant_set.contains(*n)).count();

        total_retrieved += retrieved.len();
        total_relevant_retrieved += relevant_retrieved;
        total_relevant_expected += case.relevant_names.len();
        for row in &retrieved {
            token_cost_total += (row.summary.chars().count() + 3) / 4;
        }

        let health = store.memory_health().expect("memory health");
        total_contradicted += health.contradicted_count;
        total_candidates += case.candidates.len() as i64;

        let with_memory_success = if relevant_retrieved > 0 {
            case.success_if_relevant_retrieved
        } else {
            case.baseline_success
        };
        with_memory_successes += if with_memory_success { 1.0 } else { 0.0 };
        baseline_successes += if case.baseline_success { 1.0 } else { 0.0 };
    }

    let cases_run = fixture.len();
    ReplayReport {
        cases_run,
        precision: if total_retrieved == 0 {
            1.0
        } else {
            total_relevant_retrieved as f64 / total_retrieved as f64
        },
        useful_recall: if total_relevant_expected == 0 {
            1.0
        } else {
            total_relevant_retrieved as f64 / total_relevant_expected as f64
        },
        contradiction_rate: if total_candidates == 0 {
            0.0
        } else {
            total_contradicted as f64 / total_candidates as f64
        },
        token_cost_total,
        task_delta: (with_memory_successes / cases_run as f64) - (baseline_successes / cases_run as f64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frozen expected metrics for `default_fixture()`. If this fails after
    /// an intentional fixture change, recompute the expected numbers by
    /// hand (see the comments in `default_fixture` siblings) and update
    /// both together — that hand-recomputation is what keeps this replay
    /// trustworthy as evidence rather than a number nobody checked.
    #[test]
    fn default_fixture_replay_matches_frozen_metrics() {
        let report = run_replay(&default_fixture());

        assert_eq!(report.cases_run, 4);
        assert_eq!(report.precision, 1.0, "every retrieved candidate was ground-truth relevant");
        assert_eq!(report.useful_recall, 1.0, "every ground-truth relevant candidate was retrieved");
        assert!(
            (report.contradiction_rate - (2.0 / 6.0)).abs() < 1e-9,
            "2 of 6 proposed candidates were contradicted: {}",
            report.contradiction_rate
        );
        assert_eq!(report.token_cost_total, 11, "ceil(23/4) + ceil(20/4) = 6 + 5");
        assert_eq!(report.task_delta, 0.5, "avg(1,1,1,1) - avg(0,1,0,1) = 1.0 - 0.5");
    }

    #[test]
    fn duplicate_candidate_is_never_retrieved() {
        let report = run_replay(&default_fixture());
        // A contamination regression would inflate total_retrieved beyond 2
        // (fact_a + ep_a) since the duplicate/contradicted/low-confidence
        // candidates must never reach 'promoted' status.
        assert_eq!(
            report.precision, 1.0,
            "a regression letting rejected candidates through would drop precision below 1.0"
        );
    }
}
