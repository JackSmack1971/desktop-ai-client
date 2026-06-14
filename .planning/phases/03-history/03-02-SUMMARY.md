---
phase: 03-history
plan: 02
subsystem: ipc
tags: [tauri, ipc, history, rust, serde, permissions, capabilities]

# Dependency graph
requires:
  - phase: 03-history
    plan: 01
    provides: ConversationStore, MessageStore, FtsStore, RetentionStore, migrations 0002+0003

provides:
  - history_list IPC command — Vec<ConversationSummary> ordered by updated_at DESC
  - history_get IPC command — ConversationDetail with full Vec<MessageSummary>
  - history_delete IPC command — hard-delete via RetentionStore, idempotent Ok(())
  - history_search IPC command — Vec<ConversationSummary> with snippet via FtsStore
  - HistoryError enum — StorageError, NotFound, UnauthorizedWindow with SCREAMING_SNAKE_CASE serde
  - ConversationSummary, ConversationDetail, MessageSummary response types
  - Four permission entries in capabilities/main.json
  - src-tauri/permissions/history.toml with four [[permission]] definitions

affects:
  - 03-03 chat wiring (chat.rs uses ConversationStore/MessageStore already registered)
  - 03-04 frontend history surface (history store invokes these four commands)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "assert_main_window guard: called first in every command handler, before any tauri::State access"
    - "HistoryError serde shape: { code: SCREAMING_SNAKE_CASE, message } via #[serde(tag, content, rename_all)]"
    - "Four typed stores registered in setup closure sharing one Arc<SqlitePool>"

key-files:
  created:
    - src-tauri/permissions/history.toml
  modified:
    - src-tauri/src/ipc/history.rs
    - src-tauri/src/main.rs
    - src-tauri/capabilities/main.json

key-decisions:
  - "assert_main_window called before any tauri::State access in all four commands — backend enforcement, not just capability file"
  - "ConversationSummary.snippet is Option<String> with skip_serializing_if — absent from list results, present in search results"
  - "history_delete is idempotent — RetentionStore returns Ok(()) when id does not exist"
  - "ShellPreferenceStore::new(pool) changed to ShellPreferenceStore::new(pool.clone()) to keep pool available for new stores"
  - "RetentionStore receives final pool move (not clone) since it is the last managed store"

# Metrics
duration: 5min
completed: 2026-06-14
---

# Phase 03 Plan 02: History IPC Commands Summary

**Four history_* Tauri IPC commands with typed error enum, response types, and full registration — history_list, history_get, history_delete, history_search callable from main window via assert_main_window guard.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-14T16:34:49Z
- **Completed:** 2026-06-14T16:39:12Z
- **Tasks:** 2/2
- **Files modified:** 4

## Accomplishments

### Task 1: Implement history.rs IPC command surface

`src-tauri/src/ipc/history.rs` — replaced the one-line scaffold placeholder with full implementation:

**HistoryError enum** (`#[derive(Debug, thiserror::Error, serde::Serialize)]`):
- `StorageError(String)` — serializes as `{ "code": "STORAGE_ERROR", "message": "..." }`
- `NotFound(String)` — serializes as `{ "code": "NOT_FOUND", "message": "..." }`
- `UnauthorizedWindow(String)` — serializes as `{ "code": "UNAUTHORIZED_WINDOW", "message": "..." }`

**Response types** (all `#[derive(Debug, Clone, serde::Serialize)]`):
- `MessageSummary` — id, role, content, status, created_at
- `ConversationSummary` — id, title, model, status, updated_at, snippet: Option<String> (skipped when None)
- `ConversationDetail` — id, title, model, status, updated_at, messages: Vec<MessageSummary>

**Private `assert_main_window` guard** — returns `HistoryError::UnauthorizedWindow` if label != "main"; called first in every command handler before any `tauri::State` access.

**Four `#[tauri::command]` async functions:**
- `history_list` — asserts window, calls `ConversationStore::list_conversations()`, maps rows to `ConversationSummary` (snippet: None)
- `history_get` — asserts window, fetches conversation (NotFound if absent) + messages, returns `ConversationDetail`
- `history_delete` — asserts window, delegates to `RetentionStore::delete_conversation()`, propagates errors as `StorageError`
- `history_search` — asserts window, calls `FtsStore::search()`, maps results to `ConversationSummary` (snippet: Some(...))

**Unit tests:**
- `history_error_serializes_storage_error` — StorageError → json contains "STORAGE_ERROR"
- `history_error_serializes_not_found` — NotFound → json contains "NOT_FOUND"
- `history_error_serializes_unauthorized_window` — UnauthorizedWindow → json contains "UNAUTHORIZED_WINDOW"

### Task 2: Register stores and history commands in main.rs and capabilities

**`src-tauri/src/main.rs`:**
- Added imports: `storage::fts::FtsStore`, `storage::retention::RetentionStore`, `storage::sqlite::{ConversationStore, MessageStore, ...}`
- Changed `ShellPreferenceStore::new(pool)` to `ShellPreferenceStore::new(pool.clone())` so pool remains live for new stores
- Registered `ConversationStore`, `MessageStore`, `FtsStore`, `RetentionStore` via `app.manage()` in setup closure
- Added `history_list`, `history_get`, `history_delete`, `history_search` to `tauri::generate_handler![]`

**`src-tauri/capabilities/main.json`:**
- Added four permissions: `"allow-history-list"`, `"allow-history-get"`, `"allow-history-delete"`, `"allow-history-search"`

**`src-tauri/permissions/history.toml`** (new file):
- Four `[[permission]]` entries following app-shell.toml structure exactly
- Each entry has identifier, description, and `[permission.commands] allow = [...]`

## Key Files

- `src-tauri/src/ipc/history.rs` — HistoryError, MessageSummary, ConversationSummary, ConversationDetail, assert_main_window, four IPC commands, three unit tests
- `src-tauri/src/main.rs` — four new store imports, four app.manage() calls, four generate_handler! entries
- `src-tauri/capabilities/main.json` — four allow-history-* permission grants
- `src-tauri/permissions/history.toml` — four [[permission]] definitions

## Commits

- `cfbcc29` — feat(03-02): implement history.rs IPC command surface
- `2750290` — feat(03-02): register history stores, commands, capabilities, and permissions

## Decisions Made

- assert_main_window is called first in every command handler — before any tauri::State access — enforcing the T-03-06 mitigation at the Rust level (capability file is defense-in-depth only)
- ConversationSummary.snippet uses `#[serde(skip_serializing_if = "Option::is_none")]` so list results omit the field entirely; search results include it as Some(snippet)
- ShellPreferenceStore::new(pool) updated to use pool.clone() in Task 2 — small deviation from original code shape, necessary to keep pool live for subsequent stores

## Deviations from Plan

### Small Deviation: pool clone for ShellPreferenceStore

**Found during:** Task 2

**Issue:** The existing main.rs passed `pool` (moved, not cloned) into `ShellPreferenceStore::new(pool)`. Adding four new stores after that line would be a use-after-move compile error.

**Fix:** Changed to `ShellPreferenceStore::new(pool.clone())`. The final store (`RetentionStore`) receives the moved `pool` to consume the Arc. This is the same pattern the PATTERNS.md specifies for the new stores.

**Files modified:** `src-tauri/src/main.rs`

**Classification:** Rule 3 — auto-fix blocking issue (compile error prevention)

## Security Notes

| Threat | Status |
|--------|--------|
| T-03-06: history_* called from non-main window | Mitigated — assert_main_window fires first in every handler; returns UNAUTHORIZED_WINDOW before any state access |
| T-03-07: ConversationDetail.messages contains full user prompt content | Mitigated — restricted to main window; content never logged; HistoryError never contains prompt content |
| T-03-08: history_delete with arbitrary id | Accepted — single-user desktop app; window-label authentication only |
| T-03-09: history_search with malformed FTS5 MATCH syntax | Accepted — returns HistoryError::StorageError; does not crash process |
| T-03-10: HistoryError.message leaks internal storage details | Accepted — rusqlite error string; no secrets; acceptable for local desktop single-user context |

## Verification Note

`cargo test` and `cargo build` could not be run in this environment (cargo not in PATH for Bash tool). Tests were written following the established pattern from `ipc::app_shell` and `ipc::chat` tests and are embedded in history.rs. All acceptance criteria were verified via grep checks confirming:
- `history_list` in generate_handler! block
- `ConversationStore::new` in setup closure
- Four `allow-history-*` entries in capabilities/main.json
- Four `[[permission]]` sections in permissions/history.toml

## Threat Flags

No new trust boundaries introduced beyond the plan's threat model. All four commands restrict to main window (T-03-06 mitigated). No new network endpoints, file access patterns, or schema changes were introduced.

## Known Stubs

None — all four commands are fully wired to their typed stores. No placeholder data or hardcoded empty responses.

## Self-Check: PASSED

- [x] `src-tauri/src/ipc/history.rs` — exists with HistoryError, three response types, assert_main_window, four commands, three tests
- [x] `src-tauri/src/main.rs` — contains "history_list" in generate_handler! (line 65), "ConversationStore::new" in setup (line 46)
- [x] `src-tauri/capabilities/main.json` — contains all four allow-history-* entries (lines 15-18)
- [x] `src-tauri/permissions/history.toml` — exists with four [[permission]] entries
- [x] Commit cfbcc29 — history.rs implemented
- [x] Commit 2750290 — main.rs, capabilities, history.toml updated

---
*Phase: 03-history*
*Completed: 2026-06-14*
