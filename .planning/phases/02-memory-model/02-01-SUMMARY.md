---
phase: 02-memory-model
plan: 01
subsystem: database
tags: [sqlite, rusqlite, serde_json, memory-engine, migrations]

# Dependency graph
requires:
  - phase: 01-single-agent-core
    provides: SqlitePool, ConversationStore, the conversation/turn transaction protocol that memory_run_traces references
provides:
  - "memory_candidates.tags column (migration 0007, additive)"
  - "MemoryStore::propose_candidate(..., tags: &[String]) and CandidateRow.tags: Vec<String>"
  - "docs/memory-loop.md read-only SQL inspection section for memory_* tables"
affects: [03-run-loop, 04-retrieval-promotion-verification, 05-consolidation-observability-guardrails]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Additive ALTER TABLE ... ADD COLUMN ... DEFAULT for backward-compatible schema growth, with the migration-count test renamed/retargeted rather than mutating an existing migration entry"
    - "JSON-array-string column for caller-supplied free-form lists (tags), serialized/parsed through small private helpers using the already-present serde_json dependency"

key-files:
  created: []
  modified:
    - src-tauri/src/storage/migrations.rs
    - src-tauri/src/storage/memory.rs
    - src-tauri/src/telemetry/memory_replay.rs
    - docs/memory-loop.md

key-decisions:
  - "Tags stored as a JSON array string in a new TEXT NOT NULL DEFAULT '[]' column, using serde_json (already a Cargo.toml dependency) rather than hand-rolled encoding or a new dependency"
  - "propose_candidate's tags parameter is a required trailing &[String] (not Option), per the plan's discretion note; all call sites updated explicitly rather than left to a default"
  - "Inspection documentation extends docs/memory-loop.md in a new section rather than creating a new file, matching the plan's recommended placement and avoiding a docs/README.md index update"

requirements-completed: []

# Metrics
duration: 13min
completed: 2026-06-21
status: complete
---

# Phase 2 Plan 1: Memory Model Summary

**Added a caller-supplied `tags` column to `memory_candidates` via additive migration 0007, threaded it through `MemoryStore::propose_candidate`/`CandidateRow`/`bounded_retrieve`, and documented read-only SQL inspection queries scoped to `memory_*` columns only.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-06-21T12:28:14Z
- **Completed:** 2026-06-21T12:41:28Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Migration 0007 adds `memory_candidates.tags TEXT NOT NULL DEFAULT '[]'` additively; migration 0006's literal is byte-for-byte unchanged (verified via `git diff`).
- `propose_candidate` accepts a trailing `tags: &[String]`, serializes it as JSON, and persists it in both the duplicate-rejection and new-candidate INSERT branches; `CandidateRow.tags: Vec<String>` round-trips through `bounded_retrieve` for both non-empty and empty tag lists (new unit test).
- `docs/memory-loop.md` gained an "Inspecting stored memory metadata (read-only)" section with explicit-column SELECTs against `memory_candidates` (incl. `tags`), `memory_decisions`, and `memory_run_traces`, run via `sqlite3 -readonly`, with no new Rust/IPC/UI surface.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add additive tags column (migration 0007) and persist/read it through MemoryStore** - `f8cbaf8` (feat)
2. **Task 2: Document read-only memory inspection queries proven to expose only memory_* columns** - `16ac0e3` (docs)

**Plan metadata:** _pending final commit_

## Files Created/Modified
- `src-tauri/src/storage/migrations.rs` - Appended migration 0007 (additive `ADD COLUMN tags`); renamed `migrations_count_is_six` to `migrations_count_is_seven` (asserts 7)
- `src-tauri/src/storage/memory.rs` - Added `serialize_tags`/`parse_tags` helpers, `CandidateRow.tags`, `propose_candidate`'s trailing `tags` parameter (both INSERT branches), updated `bounded_retrieve` SELECTs and `row_to_candidate`, updated all 22 existing test call sites to pass `&[]`, added `tags_round_trip_through_propose_and_retrieve`
- `src-tauri/src/telemetry/memory_replay.rs` - Updated the one production `propose_candidate` call site to pass `&[]` (no tag source yet in the replay fixture)
- `docs/memory-loop.md` - Added the read-only inspection section after "Phase 1 implementation status (shadow mode)"

## Decisions Made
- Encoded tags as a JSON array string via `serde_json` (already a dependency) rather than hand-encoding or comma-joining — simpler, unambiguous round-trip, no new dependency (per threat T-02-SC's mitigation).
- Required trailing `&[String]` parameter on `propose_candidate` instead of optional/defaulted, per the plan's explicit-write-path preference — all call sites (1 production, 22 test) updated rather than silently defaulted.
- Extended `docs/memory-loop.md` in place instead of creating a new doc file, since that file already owns the candidate record shape and the "Phase 1 implementation status" section it logically follows.

## Deviations from Plan

None - plan executed exactly as written. The plan's task instructions, file lists, and acceptance criteria were followed directly; no Rule 1-4 fixes were needed beyond the planned work itself.

## Issues Encountered

A first-pass automated edit to append `&[])` to the 22 existing test call sites in `memory.rs` introduced a literal backslash character (`\&[]`) due to a Python regex backreference escaping mistake. This was caught immediately by re-grepping the call sites before running any build/test command, and corrected with a second pass before compiling — no broken state was ever committed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 2's two remaining gaps (tags, inspection) are closed; all four phase success criteria are now satisfied (three were already satisfied by migration 0006 prior to this plan).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` (191 tests), the migration count test, the new tags round-trip test, and `cargo run --bin memory-replay` (frozen metrics unchanged: precision 1.0, useful_recall 1.0, contradiction_rate 0.333, token_cost_total 11, task_delta 0.5) all pass.
- Shadow-mode invariant intact: `grep -rn 'memory::' src-tauri/src/ipc/` returns 0 lines.
- No blockers for Phase 3 (Run Loop) or later retrieval/promotion phases; `tags` is available on `CandidateRow` for any future retrieval filtering work, though nothing in this phase wires tag-based filtering into `bounded_retrieve` (out of scope per D-01/D-02).

---
*Phase: 02-memory-model*
*Completed: 2026-06-21*

## Self-Check: PASSED

All created/modified files exist on disk; both task commits (`f8cbaf8`, `16ac0e3`) found in git log.
