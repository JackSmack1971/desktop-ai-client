# Plan 003: Order history search by FTS relevance

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report - do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat aea7052..HEAD -- src-tauri/src/ipc/history.rs src-tauri/src/storage/fts.rs`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: 005
- **Category**: bug
- **Planned at**: commit `aea7052`, 2026-06-21
- **Issue**: not published

## Why this matters

The history search command claims to return results ordered by FTS relevance, but the storage query currently sorts by conversation recency. That means the newest conversation can overshadow the best match, which makes search feel wrong even when the text match is strong.

## Current state

- `src-tauri/src/ipc/history.rs` documents `history_search` as ordered by relevance / FTS rank.
- `src-tauri/src/storage/fts.rs` still orders the query by `c.updated_at DESC`.
- The query already computes a `snippet`, so the result path is clearly intended to be search-driven rather than recency-driven.

Current code excerpts:

- `src-tauri/src/ipc/history.rs:175-179`
  - `Returns up to 50 conversations whose messages match query, ordered by relevance (FTS5 rank).`
- `src-tauri/src/storage/fts.rs:60-83`
  - `SELECT ... snippet(messages_fts, ...)`
  - `ORDER BY c.updated_at DESC`

Repo convention to follow:

- Keep the search result shape stable. The fix should change ranking only, not the IPC response fields or the snippet format.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0 |
| Search tests | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | exit 0 |

## Scope

**In scope**

- `src-tauri/src/storage/fts.rs`
- `src-tauri/src/storage/fts.rs` tests

**Out of scope**

- `src-tauri/src/ipc/history.rs` unless the doc comment needs a tiny wording cleanup after the code fix
- `src-tauri/src/storage/sqlite.rs`
- frontend stores
- docs and planning files

## Git workflow

- Branch: keep the current branch
- Commit: one query change plus one regression test
- Do not change the IPC response schema

## Steps

### Step 1: Rank search results by FTS score

Rewrite `FtsStore::search()` so the outer `ORDER BY` uses the FTS score instead of conversation recency. The minimal reliable shape is:

- compute a per-conversation FTS score with `bm25(messages_fts)`
- keep the best matching row per conversation
- sort the final rows by that score ascending, then by recency only as a tiebreaker if needed

If the bundled SQLite build does not support the query form you choose, stop and report back instead of falling back to recency ordering again.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` -> exit 0.

### Step 2: Add a relevance-order regression test

Add a storage test that creates two matching conversations where the newer one is not the better textual match. The assertion should prove the higher-relevance conversation comes back first even when it is older.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --lib` -> exit 0, including the new ranking test.

## Test plan

- Add one regression test in `src-tauri/src/storage/fts.rs`.
- Seed two conversations so one contains a stronger match signal for the same query term.
- Assert the first result is the better match, and keep the existing snippet assertion.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib` exits 0
- [ ] The top search hit is the best match, not just the newest conversation
- [ ] No files outside the in-scope list are modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- The query rewrite would require broad changes outside `src-tauri/src/storage/fts.rs`.
- The bundled SQLite version does not support the ranking query shape you use.
- The new test cannot make the relevance difference obvious with a small fixture.

## Maintenance notes

- If search pagination is added later, keep the ranking logic in the SQL layer so the page order remains stable.
- A reviewer should verify that the snippet still comes from the best matching row for each conversation.
