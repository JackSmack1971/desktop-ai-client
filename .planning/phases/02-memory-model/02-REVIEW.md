---
phase: 02-memory-model
reviewed: 2026-06-21T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src-tauri/src/storage/migrations.rs
  - src-tauri/src/storage/memory.rs
  - src-tauri/src/telemetry/memory_replay.rs
  - docs/memory-loop.md
findings:
  critical: 3
  warning: 5
  info: 4
  total: 12
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-06-21T00:00:00Z
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Reviewed the Evidence-Gated Memory Engine shadow-mode storage layer (migration 0006/0007, `MemoryStore`, the deterministic replay harness, and the accompanying doc). The shadow-mode boundary itself is well enforced (no `ipc::chat` call site reads from these tables), and the test coverage for the promotion rule and dedup logic is solid. However, the implementation has real correctness and robustness gaps: panics on malformed-but-plausible stored data instead of returning errors, a manually hand-built JSON array that bypasses the project's own `serde_json` dependency (already used two lines away) and is fragile under data containing quote characters, a release-mode-only validation gap in the migration SQL-injection guard, and a non-atomic migration bootstrap that violates this project's documented `BEGIN IMMEDIATE; ... COMMIT;` migration-transaction rule. None of these affect the live chat path today because the module is unwired, but several will surface as real bugs the moment a later phase (per `docs/memory-loop.md`) wires `bounded_retrieve` into `chat_send`, and the `debug_assert` gap is a latent injection risk that should be closed now while the blast radius is small.

## Critical Issues

### CR-01: `MemoryKind::parse` / `VerificationState::parse` panic on unexpected stored data instead of returning a recoverable error

**File:** `src-tauri/src/storage/memory.rs:41-49`, `:68-75`, and call sites at `:326`, `:329`, `:554`, `:560`
**Issue:** Both `parse` functions `panic!` on any value outside the known enum set. These are called from `decide_promotion` and `row_to_candidate`, which run against live data read back from SQLite — not validated input the caller controls. Any row written by an older/newer binary, a hand-edited row (the docs explicitly document and encourage read-only manual SQL inspection in `docs/memory-loop.md`), a partially-applied migration, or future schema drift (e.g. a CHECK constraint relaxed in a later migration without updating this Rust enum) will crash the entire process the next time `decide_promotion`, `bounded_retrieve`, or `memory_health`-adjacent paths touch that row. A single malformed row can take down the whole app, not just the memory feature — there is no isolation. This violates the "panics that crash" critical class directly, and is exactly the kind of defect that the shadow-mode framing makes easy to underweight today but will matter once this is wired into `chat_send` per the documented upgrade path.
**Fix:** Return `rusqlite::Error::FromSqlConversionFailure` (or a domain error) instead of panicking:
```rust
fn parse(s: &str) -> Result<Self, rusqlite::Error> {
    match s {
        "factual" => Ok(MemoryKind::Factual),
        "episodic" => Ok(MemoryKind::Episodic),
        "procedural" => Ok(MemoryKind::Procedural),
        "caution" => Ok(MemoryKind::Caution),
        other => Err(rusqlite::Error::InvalidColumnType(
            0,
            format!("unknown memory kind: {other:?}"),
            rusqlite::types::Type::Text,
        )),
    }
}
```
and propagate via `?` at the two call sites (`decide_promotion`, `row_to_candidate`) instead of unwrapping/panicking.

### CR-02: Hand-built JSON array in `bounded_retrieve` is unescaped and bypasses the already-imported `serde_json`

**File:** `src-tauri/src/storage/memory.rs:485-488`
**Issue:**
```rust
let ids_json = format!(
    "[{}]",
    rows.iter().map(|r| format!("\"{}\"", r.id)).collect::<Vec<_>>().join(",")
);
```
This constructs JSON by string interpolation instead of using `serde_json::to_string`, which is already a dependency used two functions away (`serialize_tags`/`parse_tags`). `r.id` is sourced from `Uuid::new_v4().to_string()` today, so it's safe in practice, but the column is typed `TEXT` with no format constraint at the SQL layer, and `CandidateRow.id` is a plain `String` with no invariant enforced at the type level. If `id` generation ever changes (e.g. test fixtures, imported data, or a future migration that allows caller-supplied IDs) and a value contains a `"` or `\`, this produces invalid JSON silently written to `memory_retrieval_log.returned_candidate_ids`, corrupting the only audit trail the shadow-mode replay tooling relies on for precision/recall measurement. This is a maintainability/data-integrity defect, not a remote-exploitable injection (ids are not externally controlled in Phase 1), but it directly contradicts the principle the same file already follows for `tags`.
**Fix:**
```rust
let ids_json = serde_json::to_string(
    &rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()
).expect("Vec<&str> always serializes to JSON");
```

### CR-03: Migration ID SQL-injection guard is compiled out in release builds

**File:** `src-tauri/src/storage/migrations.rs:318-333`
**Issue:** The only protection against unsafe `format!`-interpolated SQL (`migration.id` is spliced directly into the `SAVEPOINT migration_{id}` statement text) is:
```rust
debug_assert!(
    migration.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
    ...
);
```
`debug_assert!` is a no-op in `--release` builds (the build profile Tauri uses for the shipped app). All current `MIGRATIONS` entries are static numeric-string literals, so this is not exploitable today — but the comment immediately above ("SAFETY: migration.id and migration.sql are &'static str literals... do not generalize this pattern to dynamic values") signals the authors know this is a sharp edge, yet the only enforcement mechanism is stripped from the exact build that ships to users. A future contributor adding a migration with, say, an id copy-pasted with a stray space or a non-ASCII character would get silent malformed SQL in production with zero diagnostic, since the validation that would have caught it never runs in release.
**Fix:** Use `assert!` (not `debug_assert!`) so the check runs in all build profiles — the cost is negligible (it runs once per migration at startup) and the guard is precisely the kind of correctness floor that should not depend on build profile:
```rust
assert!(
    migration.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
    "migration id must be alphanumeric/underscore only: {:?}",
    migration.id
);
```

## Warnings

### WR-01: `schema_migrations` bootstrap and per-migration tracking writes are not transactionally atomic with the migration itself

**File:** `src-tauri/src/storage/migrations.rs:289-350`
**Issue:** `backend.md` requires: "Wrap every schema migration in an explicit `BEGIN IMMEDIATE; ... COMMIT;` block with a paired `ROLLBACK;` path in the runner." Here, the migration SQL runs inside a `SAVEPOINT`/`RELEASE SAVEPOINT` (good), but the subsequent `INSERT INTO schema_migrations ...` (line 337) executes as a separate, non-transactional `conn.execute` call after the savepoint has already been released. If the process crashes or the connection is lost between the `RELEASE SAVEPOINT` succeeding and the tracking-table `INSERT` completing, the migration's DDL is durably applied but `schema_migrations` has no record of it — the next `run_migrations` call will see `already_applied == false` and attempt to re-run already-applied DDL. Most of the migrations use `CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`, which tolerates a second run, but migration 0005 and 0007 use bare `ALTER TABLE ... ADD COLUMN` with no `IF NOT EXISTS` guard (SQLite's `ALTER TABLE ADD COLUMN` has no such clause) — a re-run after this exact crash window will throw `duplicate column name` and the app will fail to start entirely.
**Fix:** Wrap the savepoint and the tracking-row insert in the same outer transaction, or at minimum write the tracking row for success inside the same savepoint block before `RELEASE`:
```rust
let result = conn.execute_batch(&format!(
    "SAVEPOINT migration_{id};\n{sql}\nRELEASE SAVEPOINT migration_{id};",
    id = migration.id, sql = migration.sql,
));
// existing tracking insert stays separate for the failure path (so a
// failed migration is still recorded), but consider making ADD COLUMN
// migrations idempotent regardless, e.g. by checking pragma_table_info
// before ALTER TABLE, since the tracking-row write can never be made
// perfectly atomic with a DDL statement that already auto-committed.
```
At minimum, make the `ALTER TABLE ADD COLUMN` migrations (0005, 0007) idempotent by checking `pragma_table_info` first, since re-application is the documented recovery behavior this runner relies on.

### WR-02: `parse_tags` panics on malformed JSON instead of returning an error

**File:** `src-tauri/src/storage/memory.rs:196-198`
**Issue:**
```rust
fn parse_tags(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_else(|_| panic!("invalid tags JSON in storage: {raw:?}"))
}
```
Same class of issue as CR-01: this is read-back data, not validated input. The column default is `'[]'` and all writes go through `serialize_tags`, so under normal operation this can't fail — but `docs/memory-loop.md` explicitly documents and encourages direct read-only `sqlite3` inspection of `memory_candidates`, and the schema has no CHECK constraint enforcing `tags` is valid JSON. A hand-edited or externally-imported row crashes the process on next retrieval.
**Fix:** Propagate via `rusqlite::Result` like CR-01's fix, or at minimum fall back to an empty vec with a logged warning rather than panicking, consistent with how `bounded_retrieve` is meant to degrade gracefully (it's read-only retrieval, not a write path where failing loud is more defensible).

### WR-03: `dedup_key`'s case/whitespace normalization can silently merge semantically distinct facts across kinds with shared casing quirks

**File:** `src-tauri/src/storage/memory.rs:184-187`
**Issue:** Not a bug in the strict sense — this is documented as "exact-match only" — but the normalization (`split_whitespace().join(" ").to_lowercase()`) means two factual candidates like `"The limit is 60rpm."` (with trailing punctuation) and `"the limit is 60rpm"` (without) are treated as distinct dedup keys (punctuation isn't stripped), which is inconsistent with the stated intent ("the rate limit is sixty" / "THE RATE LIMIT IS SIXTY" collapse in the fixture, but punctuation differences won't). This is a quality gap relative to the documented behavior, not a crash risk.
**Fix:** Either document the punctuation-sensitivity limitation explicitly next to the function (the current doc comment implies more normalization than is actually performed), or extend normalization to strip trailing punctuation if that's the intended dedup granularity.

### WR-04: `expire_stale` issues one UPDATE and one INSERT per expired row inside a single transaction, with no batching ceiling

**File:** `src-tauri/src/storage/memory.rs:417-442`
**Issue:** This is flagged as a warning rather than performance-out-of-scope because it's a correctness-adjacent concern: the whole sweep runs inside one `with_transaction`, so on a database with a large backlog of stale candidates (e.g. after the app has been offline and `expire_stale` hasn't run in a long time), this transaction holds the connection mutex (`SqlitePool::with_transaction` takes the single shared `Mutex<Connection>`) for the entire sweep, blocking all other reads/writes — including any concurrent chat turn writes — until it completes. There's no LIMIT/chunking.
**Fix:** Cap the sweep batch size per call (e.g. `LIMIT 500`) and document that callers should loop until 0 rows are returned, so a large backlog doesn't monopolize the single connection mutex in one transaction.

### WR-05: `MemoryStore::memory_health` issues 7 sequential queries with no transactional consistency guarantee

**File:** `src-tauri/src/storage/memory.rs:506-545`
**Issue:** Uses `with_conn` (not `with_transaction`), and each `query_row` call is a separate statement against the connection. Since `with_conn` only holds the mutex for the duration of the single closure call — but the closure itself runs all 7 queries sequentially without an explicit `BEGIN`/transaction wrapping them — concurrent writers (if this is ever called from a background task while `propose_candidate`/`decide_promotion` are also running, which the doc comment for this function anticipates: "a future `memory_health` IPC command") could observe inconsistent counts across the 7 queries (e.g. `candidate_count` reflects state before a promotion that `promoted_count` then also reflects, double-counting or undercounting a row that transitioned mid-read). Today this is purely diagnostic and not wired to any command, so the practical risk is zero, but the function will silently carry this inconsistency into whatever future caller wires it up unless flagged now.
**Fix:** Either wrap the body in `with_transaction` (read-only `BEGIN DEFERRED` still gives snapshot isolation under SQLite's locking model) or note in the doc comment that counts are not point-in-time consistent under concurrent writers.

## Info

### IN-01: `MemoryKind`/`VerificationState` round-trip via `&str` instead of `rusqlite::types::{FromSql, ToSql}`

**File:** `src-tauri/src/storage/memory.rs:31-50, 59-76`
**Issue:** Both enums implement manual `as_str`/`parse` pairs called explicitly at every query site (`kind.as_str()`, `MemoryKind::parse(&kind)`) rather than implementing `rusqlite::types::FromSql`/`ToSql`, which would let them be bound/read directly as `params![kind]` and `row.get::<_, MemoryKind>(1)?`. This is a style/maintainability observation, not a bug — the current pattern works — but it duplicates the same boilerplate at every call site and is easy to forget when adding a new query.
**Fix:** Consider implementing `FromSql`/`ToSql` for both enums (returning `FromSqlError::Other` instead of panicking, which would also resolve CR-01) to remove the repeated manual conversion at each of the 4+ call sites.

### IN-02: `Decision::Skipped` variant requires an `unreachable!()` in `promotion_rule`'s caller, indicating the enum carries cases that don't apply uniformly

**File:** `src-tauri/src/storage/memory.rs:333-338`
**Issue:**
```rust
let (new_status, action, reason): (&str, &str, &'static str) = match decision {
    Decision::Promoted => ("promoted", "promoted", "promotion_rule_satisfied"),
    Decision::Rejected(reason) => ("rejected", "rejected", reason),
    Decision::Deferred(reason) => ("candidate", "deferred", reason),
    Decision::Skipped => unreachable!("promotion_rule never returns Skipped"),
};
```
`Decision` is shared between `promotion_rule`'s return type and `decide_promotion`'s return type, but only 3 of the 4 variants are reachable from `promotion_rule`. This works correctly today (verified — `decide_promotion` returns `Decision::Skipped` directly at lines 319/322 before ever calling `promotion_rule`), but it's a type that doesn't accurately model the function it's passed through, relying on a documented-but-unenforced invariant.
**Fix:** Split into two types — a `PromotionRuleOutcome` enum with 3 variants returned by `promotion_rule`, and the existing 4-variant `Decision` for the public `decide_promotion` API — removing the need for `unreachable!()`.

### IN-03: `docs/memory-loop.md` "Why this is safe to expose" section names `attachments` as an excluded table, but no `attachments` table exists in `migrations.rs`

**File:** `docs/memory-loop.md:181-182`, cross-referenced against `src-tauri/src/storage/migrations.rs:33-278`
**Issue:** The doc states the inspection queries never join to `"messages", "conversations", "turns", "turn_attempts", "attachments", or "artifacts"`, but no migration in this file creates an `attachments` table — only `conversations`, `messages`, `artifacts`, `turns`, `turn_attempts`, and the `memory_*`/`shell_preferences` tables exist here. This may be accurate if `attachments` is defined in a migration file outside this phase's scope, but within the reviewed file set it's an unverifiable/dangling reference that should be confirmed or removed to keep the privacy-boundary claim auditable against this file alone.
**Fix:** Verify `attachments` exists in a different migrations file/module; if so, add a one-line pointer (e.g. "see migration NNNN") so the claim is traceable from this doc.

### IN-04: `mark_contradiction`'s symmetric loop comment claims "defense in depth via `promotion_rule`'s own contradiction_state check" but that path is dead given the surrounding status guard

**File:** `src-tauri/src/storage/memory.rs:824-829` (test comment), cross-referenced against `:321-323` and `:146-148`
**Issue:** The test comment for `mark_contradiction_rejects_both_candidates` asserts that `promotion_rule`'s `contradiction_state == "contradicted"` check (line 146) is "defense in depth," but `decide_promotion` already returns `Decision::Skipped` early at line 321-323 whenever `status != "candidate"` — and `mark_contradiction` always sets `status = 'rejected'` in the same statement that sets `contradiction_state = 'contradicted'` (lines 363-365), so `promotion_rule` can never actually observe `contradiction_state == "contradicted"` while `status == "candidate"` through any code path in this module. The check at line 146-148 is currently unreachable dead code given the only caller of `mark_contradiction`. This isn't harmful (it is genuinely defensive against a hypothetical future caller that sets `contradiction_state` without also setting `status`), but the test comment overstates it as exercised "defense in depth" when no test actually reaches that branch.
**Fix:** Either add a direct unit test for `promotion_rule(... contradiction_state: "contradicted", ...)` with `status` left as `'candidate'` (requires a test-only path that bypasses `mark_contradiction`, e.g. calling `promotion_rule` directly, which the test file already imports it for), or soften the comment to acknowledge the branch is currently unreached through the public API.

---

_Reviewed: 2026-06-21T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
