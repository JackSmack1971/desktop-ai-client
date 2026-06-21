# Plan 001: Restore the storage wrapper so the backend compiles again

> **Superseded**: this helper-only fix was abandoned after execution showed it was under-scoped. Use plan 005 instead.

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report - do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat aea7052..HEAD -- src-tauri/src/storage/sqlite.rs src-tauri/src/storage/memory.rs src-tauri/src/storage/retention.rs`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `aea7052`, 2026-06-21
- **Issue**: not published

## Why this matters

The backend currently fails to compile because `SqlitePool::with_conn` and `SqlitePool::with_transaction` were changed to return `StorageError` while the rest of the storage layer still expects `rusqlite::Result`. That breaks `cargo check`, so the app cannot ship and no later plan can be verified cleanly until the storage wrapper contract is restored.

## Current state

- `src-tauri/src/storage/sqlite.rs` defines `StorageError` and now returns `Result<T, StorageError>` from both `with_conn` and `with_transaction`.
- `src-tauri/src/storage/memory.rs` still returns `rusqlite::Result` from its public API and calls those helpers with `?`, which is what triggers the compile failures.
- `src-tauri/src/storage/retention.rs` has the same mismatch.
- `cargo check --manifest-path src-tauri/Cargo.toml` currently fails with `E0277` and `E0308` in those storage call sites.

Current code excerpts:

- `src-tauri/src/storage/sqlite.rs:70-96`
  - `pub fn with_conn<F, T>(&self, f: F) -> Result<T, StorageError>`
  - `pub fn with_transaction<F, T>(&self, f: F) -> Result<T, StorageError>`
- `src-tauri/src/storage/memory.rs:214-506`
  - public methods such as `record_run_trace`, `propose_candidate`, `decide_promotion`, `mark_contradiction`, `record_reuse`, `expire_stale`, `bounded_retrieve`, and `memory_health` still return `rusqlite::Result`.
- `src-tauri/src/storage/retention.rs:38-51`
  - `pub fn delete_conversation(&self, id: &str) -> rusqlite::Result<()>`
  - it delegates through `self.pool.with_conn(...)`.

Repo convention to follow:

- Storage modules already expose typed domain APIs and let the IPC layer convert storage errors at the boundary. Keep this plan narrow and restore the existing storage-layer compatibility first; do not spread a cross-crate error migration into unrelated modules.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0, no `E0277`/`E0308` |
| Storage tests | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | exit 0 |

## Scope

**In scope**

- `src-tauri/src/storage/sqlite.rs`
- `src-tauri/src/storage/memory.rs`
- `src-tauri/src/storage/retention.rs`

**Out of scope**

- `src-tauri/src/ipc/`
- `src-tauri/src/providers/`
- `src/lib/`
- migration SQL
- docs and planning files

## Git workflow

- Branch: keep the current branch
- Commit: one logical fix only, no unrelated cleanup
- Do not push or open PRs

## Steps

### Step 1: Restore the storage API contract

Make `SqlitePool::with_conn` and `SqlitePool::with_transaction` compatible with the callers that already exist. The lazy, correct fix is to return `rusqlite::Result<T>` from those helpers again so the storage layer compiles without a wider error-type migration.

Do not migrate `memory.rs` or `retention.rs` to `StorageError` in this plan. The goal is to get the backend compiling again, not to redesign the storage error surface.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` -> exit 0 and the previous `E0277` / `E0308` errors are gone.

### Step 2: Re-run the storage tests

Run the storage-focused test slice after the compile fix. This catches any accidental regression in the wrapper while keeping the verification narrow.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --lib` -> exit 0.

## Test plan

- No new tests are required if the compile fix is purely a signature restoration.
- If the fix needs a small bridge or adapter, keep the change inside `src-tauri/src/storage/sqlite.rs` and reuse the existing storage unit tests in that module.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib` exits 0
- [ ] No files outside the in-scope list are modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- Restoring the helper signatures would require touching IPC or provider code.
- `cargo check` still fails after the wrapper contract is restored.
- The live code no longer matches the excerpts above.

## Maintenance notes

- This is intentionally the smallest repair. If the team still wants `StorageError` end-to-end later, that should be a separate plan with IPC boundary updates and error mapping changes.
- A reviewer should verify that the compile fix does not silently change the storage error semantics seen by callers outside this subtree.
