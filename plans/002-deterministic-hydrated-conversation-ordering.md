# Plan 002: Make hydrated conversation ordering deterministic

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report - do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat aea7052..HEAD -- src-tauri/src/storage/migrations.rs src-tauri/src/storage/sqlite.rs src/lib/stores/chat.ts`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: 005
- **Category**: bug
- **Planned at**: commit `aea7052`, 2026-06-21
- **Issue**: not published

## Why this matters

The backend currently orders hydrated messages only by `created_at`, but the schema stores that timestamp at second precision. Two messages created in the same second can therefore hydrate in the wrong order, which is exactly the shape that `chatStore.retryMessage()` depends on for a reopened conversation. When that ordering slips, retry can become a no-op or can pair the assistant row with the wrong user row.

## Current state

- `src-tauri/src/storage/migrations.rs` gives `messages.created_at` a `datetime('now')` default, so the timestamp is coarse.
- `src-tauri/src/storage/sqlite.rs` orders `MessageStore::get_messages()` by `created_at ASC` only.
- `src/lib/stores/chat.ts` assumes the assistant message being retried is immediately preceded by its user message.

Current code excerpts:

- `src-tauri/src/storage/migrations.rs:59-67`
  - `CREATE TABLE IF NOT EXISTS messages (...)`
  - `created_at TEXT NOT NULL DEFAULT (datetime('now'))`
- `src-tauri/src/storage/sqlite.rs:307-313`
  - `ORDER BY created_at ASC`
- `src/lib/stores/chat.ts:253-257`
  - `if (idx <= 0) return;`
  - `const userMsg = messages[idx - 1];`

Repo convention to follow:

- The backend owns the persistence contract. Keep the fix in the Rust storage layer so the renderer continues to trust `history_get` / hydration ordering instead of adding frontend heuristics.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0 |
| Storage tests | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | exit 0 |

## Scope

**In scope**

- `src-tauri/src/storage/sqlite.rs`
- `src-tauri/src/storage/migrations.rs` only if a schema change is truly required
- `src-tauri/src/storage/sqlite.rs` tests

**Out of scope**

- `src/lib/stores/chat.ts`
- `src-tauri/src/ipc/`
- provider code
- docs and planning files

## Git workflow

- Branch: keep the current branch
- Commit: one focused storage fix
- Do not change the public chat message shape

## Steps

### Step 1: Make message ordering deterministic

Use a stable secondary sort key in `MessageStore::get_messages()` so rows with the same `created_at` value still hydrate in insertion order. The lazy fix is to keep the schema intact and order by `created_at ASC, rowid ASC`.

If `rowid` is unavailable for this table for any reason, stop and report back instead of inventing a new schema on the fly. The current schema is a normal `STRICT` table with a primary key, so `rowid` should exist today.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` -> exit 0.

### Step 2: Add a regression test for same-second rows

Add or extend a storage test in `src-tauri/src/storage/sqlite.rs` that inserts a user row and an assistant row with the same `created_at` value and asserts `get_messages()` returns them in the original turn order. The test should prove the fix without touching frontend code.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --lib` -> exit 0, including the new regression test.

## Test plan

- Add one regression test in `src-tauri/src/storage/sqlite.rs` for two same-second messages in one conversation.
- If you need to force same-second timestamps, set the `created_at` field explicitly in the test after insert so the test is deterministic.
- Keep the test aligned with the existing storage test style in the same module.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib` exits 0
- [ ] The new regression test proves same-second rows hydrate in turn order
- [ ] No files outside the in-scope list are modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- The fix would require a new migration or a new sequence column and the tradeoff is not obvious.
- The live schema no longer supports `rowid` ordering.
- The regression test cannot make same-second ordering deterministic with a small local change.

## Maintenance notes

- If the message schema ever moves to `WITHOUT ROWID` or gets a real sequence column, revisit this plan and replace the `rowid` tie-breaker with the new stable key.
- A reviewer should check that the backend still returns a turn in the exact order the renderer expects, because the retry flow depends on it.
