---
phase: 02-memory-model
verified: 2026-06-21T00:00:00Z
status: passed
score: 4/4 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 2: Memory Model Verification Report

**Phase Goal:** The system can store candidate memories separately from raw traces without letting live chat depend on them.
**Verified:** 2026-06-21
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Memory records are stored in tables/stores separate from raw conversation traces | VERIFIED | `memory_run_traces`, `memory_candidates`, `memory_decisions`, `memory_retrieval_log` are distinct SQLite tables created by migration 0006 (`src-tauri/src/storage/migrations.rs:175-269`), entirely separate from `messages`/`conversations`/`turns`. FK cascade chains back via `conversation_id`/`source_run_trace_id`, never via the `messages` table. Confirmed by passing tests `memory_candidates_and_decisions_tables_exist_after_migration`, `dropping_memory_tables_does_not_affect_core_tables`. |
| 2 | Each candidate carries type, summary, source, tags, confidence, utility, recency, verification state, expiry | VERIFIED | `memory_candidates` schema (migration 0006 + 0007) has `kind`, `summary`, `source_run_trace_id`, `confidence`, `utility`, `created_at`/`updated_at` (recency), `verification_state`, `expires_at`, and the newly added `tags` column (`TEXT NOT NULL DEFAULT '[]'`, migration 0007). `CandidateRow` (`memory.rs:80-91`) exposes all these as typed fields including `pub tags: Vec<String>`. Round-trip proven by `tags_round_trip_through_propose_and_retrieve` test (passing). |
| 3 | The memory pipeline remains shadow-mode only and does not change live provider routing or chat behavior | VERIFIED | `grep -rn 'memory::' src-tauri/src/ipc/` returns 0 lines (only unrelated hit: `Connection::open_in_memory()` string in `app_shell.rs`, not a `memory::` module reference). `memory.rs` module doc and `bounded_retrieve` doc comment explicitly state the shadow-mode invariant; only consumers are its own test module and `telemetry::memory_replay`, confirmed via direct file inspection. |
| 4 | A human can inspect stored memory metadata without exposing backend-owned secrets or raw file paths | VERIFIED | `docs/memory-loop.md` "Inspecting stored memory metadata (read-only)" section (lines 122-189) documents `sqlite3 -readonly` direct-file SQL with explicit column lists against `memory_candidates` (incl. `tags`), `memory_decisions`, `memory_run_traces`. `grep -niE 'select \* from memory_'` returns 0 matches; `grep -niE 'from (messages\|conversations\|turns\|turn_attempts\|attachments\|artifacts\|file_tokens)'` returns 0 matches. Rationale section cites `docs/privacy-boundaries.md`'s finding that memory_* tables hold only derived summaries and that secrets/paths live in the attachment/file-token intake path, which these queries never touch. |

**Score:** 4/4 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src-tauri/src/storage/migrations.rs` | Migration 0007 adding `tags` column, additive, 0006 untouched | VERIFIED | `id: "0007"` literal present (1 match); `ALTER TABLE memory_candidates ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'` present; `git show f8cbaf8` diff confirms 0006's literal block has zero changed lines — only an addition after it plus the unrelated test rename. |
| `src-tauri/src/storage/memory.rs` | `propose_candidate` persists tags; `CandidateRow` exposes tags; round-trip unit test | VERIFIED | `pub tags: Vec<String>` field present on `CandidateRow`; `propose_candidate(..., tags: &[String])` trailing parameter present on both INSERT branches; `serialize_tags`/`parse_tags` helpers present; `tags_round_trip_through_propose_and_retrieve` test present and passing. |
| `docs/memory-loop.md` | Read-only inspection query section, memory_*-only columns | VERIFIED | "Inspecting stored memory metadata (read-only)" section present with three explicit-column SELECTs (`memory_candidates` incl. `tags`, `memory_decisions`, `memory_run_traces`) and a rationale subsection. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src-tauri/src/storage/memory.rs` | `src-tauri/src/storage/migrations.rs` | `propose_candidate` INSERT and `bounded_retrieve` SELECT reference the `tags` column added by migration 0007 | WIRED | Both INSERT branches in `propose_candidate` include `tags` in the explicit column list and bind `tags_json`; both `bounded_retrieve` SELECT branches include `tags` as the last selected column, matching `row_to_candidate`'s `row.get(9)` index. |
| `src-tauri/src/telemetry/memory_replay.rs` | `src-tauri/src/storage/memory.rs` | `memory_replay` calls `propose_candidate` with its new `tags` argument | WIRED | Line 179: `propose_candidate(candidate.kind, candidate.summary, &trace_id, candidate.confidence, &[])` — the lone production call site updated to the new 5-arg signature. |

### Behavioral Spot-Checks / Test Execution

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Memory unit tests pass (incl. tags round-trip) | `cargo test --manifest-path src-tauri/Cargo.toml --lib memory` | 21 passed; 0 failed | PASS |
| Migration tests pass (incl. count-is-seven) | `cargo test --manifest-path src-tauri/Cargo.toml --lib migrations` | 15 passed; 0 failed | PASS |
| Crate builds with updated signature at all call sites | `cargo build --manifest-path src-tauri/Cargo.toml` | exit 0 | PASS |
| Shadow-mode replay harness still runs | `cargo run --manifest-path src-tauri/Cargo.toml --bin memory-replay` | precision 1.0, useful_recall 1.0, contradiction_rate 0.3333, token_cost_total 11, task_delta 0.5 (matches SUMMARY claim) | PASS |
| Shadow-mode IPC isolation | `grep -rn 'memory::' src-tauri/src/ipc/` | 0 matches | PASS |
| Inspection query safety (no SELECT *, no foreign-table joins) | `grep -niE 'select \* from memory_'` / `grep -niE 'from (messages\|conversations\|...)'` against `docs/memory-loop.md` | both 0 matches | PASS |

### Requirements Coverage

PLAN frontmatter declares `requirements: []`. `.planning/REQUIREMENTS.md` contains no v1 requirements (ingest produced zero PRD/ADR-derived requirements) and no Phase 2 traceability rows. No requirement IDs to cross-reference; nothing orphaned.

### Anti-Patterns Found

None. Scanned all four phase-modified files (`migrations.rs`, `memory.rs`, `memory_replay.rs`, `docs/memory-loop.md`) for `TBD|FIXME|XXX`, `TODO|HACK|PLACEHOLDER`, and "not yet implemented"/"coming soon" language — zero matches. Two pre-existing `ponytail:` comments in `memory.rs` (lines 136, 357) are documented, intentional deferrals from the prior phase (deterministic promotion rule vs. future LLM judge; no contradiction-reconciliation flow yet) — not unreferenced debt markers, and not part of this phase's diff (confirmed via `git show f8cbaf8`, which touches neither line).

### Human Verification Required

None. This phase is a backend storage/documentation change with no UI, IPC, or live-behavior surface — all four success criteria are mechanically verifiable via schema inspection, grep, and test execution, all of which were run directly against the codebase (not inferred from SUMMARY.md).

### Gaps Summary

No gaps. All four ROADMAP success criteria are independently verified against the codebase:
1. Separate storage — confirmed via schema + cascade tests.
2. Full candidate metadata incl. tags — confirmed via schema, `CandidateRow`, and a passing round-trip test.
3. Shadow-mode isolation — confirmed via direct grep of `src-tauri/src/ipc/`.
4. Safe read-only inspection — confirmed via the documented SQL and negative greps proving no `SELECT *` or cross-table joins.

The plan's claim that criteria 1, 2 (minus tags), and 3 were "already satisfied by migration 0006" prior to this plan was independently re-verified here rather than trusted — migration 0006's table definitions, the IPC isolation grep, and the existing test suite all confirm this was true, and migration 0006 itself was left byte-for-byte unchanged by this phase's commit.

---

*Verified: 2026-06-21*
*Verifier: Claude (gsd-verifier)*
