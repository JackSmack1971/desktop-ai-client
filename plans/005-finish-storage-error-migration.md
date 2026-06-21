# Plan 005: Finish the backend StorageError migration across storage

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report - do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat aea7052..HEAD -- src-tauri/src/storage/sqlite.rs src-tauri/src/storage/memory.rs src-tauri/src/storage/retention.rs src-tauri/src/storage/fts.rs src-tauri/src/storage/turns.rs`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: none
- **Category**: migration
- **Planned at**: commit `aea7052`, 2026-06-21
- **Issue**: not published

## Why this matters

The storage layer is currently split across two error contracts. `src-tauri/src/storage/sqlite.rs` was rolled back to `rusqlite::Result`, while `src-tauri/src/storage/fts.rs` and `src-tauri/src/storage/turns.rs` already depend on `StorageError`, and `memory.rs` / `retention.rs` still return `rusqlite::Result` from their public APIs. That leaves the backend in an in-between state where the compile baseline is broken and the storage boundary is no longer coherent.

This plan finishes the migration in one place so the backend has a single typed storage error contract again. Keep the change inside the storage subsystem unless a compiler error proves one exact IPC handler must be adjusted.

## Current state

The executor needs the live split inlined, not referenced indirectly:

- `src-tauri/src/storage/sqlite.rs` is the connection wrapper; it currently returns `rusqlite::Result` from `with_conn` and `with_transaction` again.
- `src-tauri/src/storage/memory.rs` is the memory-engine store; its public methods still return `rusqlite::Result`, even though they call the shared SQLite helpers.
- `src-tauri/src/storage/retention.rs` is the conversation-delete store; it still returns `rusqlite::Result<()>`.
- `src-tauri/src/storage/fts.rs` is already on `StorageError` and should stay aligned with the final storage contract.
- `src-tauri/src/storage/turns.rs` is already on `StorageError` and should stay aligned with the final storage contract.

Current code excerpts:

- `src-tauri/src/storage/sqlite.rs:57-88`
  - `pub fn with_conn<F, T>(&self, f: F) -> rusqlite::Result<T>`
  - `pub fn with_transaction<F, T>(&self, f: F) -> rusqlite::Result<T>`
- `src-tauri/src/storage/memory.rs:222-507`
  - public methods such as `record_run_trace`, `propose_candidate`, `decide_promotion`, `mark_contradiction`, `record_reuse`, `expire_stale`, `bounded_retrieve`, and `memory_health` still return `rusqlite::Result`
- `src-tauri/src/storage/retention.rs:38-39`
  - `pub fn delete_conversation(&self, id: &str) -> rusqlite::Result<()>`
- `src-tauri/src/storage/fts.rs:10, 64`
  - `use crate::storage::sqlite::{SqlitePool, StorageError};`
  - `pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, StorageError>`
- `src-tauri/src/storage/turns.rs:19, 84, 163, 196, 262, 301, 340, 375`
  - `use crate::storage::sqlite::{SqlitePool, StorageError};`
  - the turn lifecycle methods already return `StorageError`

Repo convention to follow:

- Storage stores are typed backend APIs, and the IPC layer is supposed to convert storage errors at the boundary. Match the already-migrated `fts.rs` and `turns.rs` contract instead of inventing a third error shape in the middle of the backend.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0, no unresolved `StorageError` imports or `E0308` type mismatches |
| Storage tests | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | exit 0 |

## Scope

**In scope**

- `src-tauri/src/storage/sqlite.rs`
- `src-tauri/src/storage/memory.rs`
- `src-tauri/src/storage/retention.rs`
- `src-tauri/src/storage/fts.rs`
- `src-tauri/src/storage/turns.rs`

**Out of scope**

- `src-tauri/src/providers/`
- `src/lib/`
- migration SQL
- docs and planning files

## Git workflow

- Branch: keep the current branch
- Commit: one logical storage-contract fix only
- Do not push or open PRs

## Steps

### Step 1: Restore the shared StorageError contract in the SQLite wrapper

Bring `SqlitePool::with_conn` and `SqlitePool::with_transaction` back to the shared storage error contract that the migrated storage modules expect. Keep the partial migration in `fts.rs` and `turns.rs` intact; the goal is to restore a single backend storage contract, not to back out the files that already moved.

If this requires introducing or restoring a small `StorageError` type inside `sqlite.rs`, do that only as far as needed to make the storage layer compile again. Do not widen the change into renderer or provider code.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` -> exit 0, and the unresolved `StorageError` import errors disappear.

### Step 2: Finish the remaining storage callers

Update `memory.rs` and `retention.rs` so their public methods propagate the same storage error type as the rest of the subsystem. Keep the edits local to the storage files above and preserve the existing behavior of the methods; only the error surface should change.

If `cargo check` exposes one or two tiny IPC mapper mismatches after the storage layer is consistent, patch only those exact handlers and stop if the fix would widen beyond a direct compile correction.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` -> exit 0.

### Step 3: Re-run the storage test slice

Run the lib test suite after the contract is consistent so the executor proves the storage behavior still holds under the migrated error surface.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --lib` -> exit 0.

## Test plan

- No new tests are required if this stays a signature-and-error-surface migration.
- Reuse the existing storage tests in `sqlite.rs`, `memory.rs`, `retention.rs`, `fts.rs`, and `turns.rs` as the regression net.
- If a tiny IPC handler patch is forced by compilation, do not add broad new tests; keep the change narrow and let `cargo check` plus the existing storage tests validate it.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib` exits 0
- [ ] `sqlite.rs`, `memory.rs`, `retention.rs`, `fts.rs`, and `turns.rs` all agree on one storage error contract
- [ ] No files outside the in-scope list are modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- Restoring the shared storage error contract would require a broad IPC or provider redesign.
- `cargo check` keeps surfacing new storage files beyond the scope listed above.
- The live code no longer matches the excerpts above.

## Maintenance notes

- Future storage changes should treat `StorageError` as the backend contract boundary, not a per-file implementation detail.
- A reviewer should verify that the migration does not leak backend-only errors across the Tauri boundary and that the storage tests still cover the same behavior after the contract change.
