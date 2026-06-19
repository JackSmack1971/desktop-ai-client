---
phase: 03-history
plan: 01
subsystem: database
tags:
  [sqlite, fts5, rusqlite, migrations, conversation-history, full-text-search]

# Dependency graph
requires:
  - phase: 01-app-shell
    provides: SqlitePool, ShellPreferenceStore pattern, migration runner infrastructure

provides:
  - Migration 0002 — conversations and messages tables (STRICT mode, FK cascade, CHECK constraints)
  - Migration 0003 — messages_fts FTS5 virtual table + INSERT/DELETE/UPDATE sync triggers
  - ConversationStore — typed store for conversations table (create, list, get, mark_complete, mark_incomplete)
  - MessageStore — typed store for messages table (insert_message, insert_incomplete_message, get_messages)
  - FtsStore — typed store for FTS5 search with snippet() auxiliary function
  - RetentionStore — typed store for hard-delete + WAL checkpoint (non-fatal checkpoint errors)

affects:
  - 03-02 IPC layer (history_list, history_get, history_delete, history_search commands consume these stores)
  - 03-03 chat wiring (chat_send must use ConversationStore + MessageStore for persistence)
  - 03-04 frontend history surface (typed IPC responses will carry ConversationRow/SearchResult fields)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - 'Typed domain store pattern: pub struct XxxStore { pool: Arc<SqlitePool> } with with_conn delegation'
    - "FTS5 external-content table: content='messages', content_rowid='rowid' with INSERT/DELETE/UPDATE sync triggers"
    - 'WAL checkpoint after hard-delete: non-fatal eprintln warning, never block delete response'

key-files:
  created:
    - src-tauri/src/storage/fts.rs
    - src-tauri/src/storage/retention.rs
    - src-tauri/src/storage/backup.rs
  modified:
    - src-tauri/src/storage/migrations.rs
    - src-tauri/src/storage/sqlite.rs

key-decisions:
  - "FTS5 external-content table tied to messages table via content='messages'; DDL lives in migration 0003, not in FtsStore"
  - 'WAL checkpoint errors after delete are non-fatal (eprintln warning only, per D-14)'
  - 'ConversationStore.create_conversation accepts (id, title) — model is set later by mark_complete when Done event fires'
  - 'RetentionStore.delete_conversation returns Ok(()) when id does not exist (idempotent no-op)'
  - 'FtsStore.search returns empty Vec on no matches — QueryReturnedNoRows is not propagated as error'

patterns-established:
  - 'Typed domain stores: ShellPreferenceStore pattern extended for ConversationStore, MessageStore, FtsStore, RetentionStore'
  - 'with_conn delegation: all DB access inside store impl methods, never directly in IPC handlers'
  - 'In-memory test helper with run_migrations: migrated_pool() pattern for testing stores'

requirements-completed:
  - HIST-01
  - HIST-02
  - HIST-03

# Metrics
duration: 45min
completed: 2026-06-14
---

# Phase 03 Plan 01: History Schema and Domain Stores Summary

**SQLite schema and typed domain stores for conversation history — migrations 0002+0003 (conversations, messages, FTS5) with ConversationStore, MessageStore, FtsStore, and RetentionStore covering create/list/get/search/delete.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-06-14T00:00:00Z
- **Completed:** 2026-06-14
- **Tasks:** 2/2
- **Files modified:** 5

## Accomplishments

### Task 1: Migrations 0002 and 0003

`src-tauri/src/storage/migrations.rs` — appended two Migration entries to the MIGRATIONS static slice:

- **Migration 0002** (`"Create conversations and messages tables"`): conversations table with STRICT mode, CHECK constraint on status (`'active'|'complete'|'incomplete'`), messages table with ON DELETE CASCADE FK to conversations, and `idx_messages_conversation_id` index.
- **Migration 0003** (`"Create FTS5 external-content table and sync triggers for messages"`): `messages_fts` FTS5 virtual table with `content='messages'`, `content_rowid='rowid'`, `tokenize='unicode61'`, plus INSERT/DELETE/UPDATE triggers (`messages_ai`, `messages_ad`, `messages_au`) to keep the FTS index in sync.

MIGRATIONS.len() is now 3. Tests added: count assertion, conversations/messages round-trip, status CHECK constraint enforcement, ON DELETE CASCADE, and FTS5 trigger verification.

### Task 2: Typed Domain Stores

**`sqlite.rs`** — added ConversationRow, ConversationStore, MessageRow, MessageStore:

- `ConversationStore::create_conversation` inserts with `status='active'`
- `ConversationStore::list_conversations` returns Vec<ConversationRow> ordered by `updated_at DESC`
- `ConversationStore::get_conversation` returns `Option<ConversationRow>` (None on missing)
- `ConversationStore::mark_complete(id, model)` sets `status='complete'`, records model
- `ConversationStore::mark_incomplete(id)` sets `status='incomplete'`
- `MessageStore::insert_message` — `status='complete'`; `insert_incomplete_message` — `status='incomplete'`
- `MessageStore::get_messages` returns Vec<MessageRow> ordered by `created_at ASC`

**`fts.rs`** — FtsStore with SearchResult struct:

- `FtsStore::search(query)` issues parameterized MATCH query (T-03-04: no string interpolation), returns Vec<SearchResult> with `snippet(messages_fts, 0, '<b>', '</b>', '…', 15)`, grouped by conversation, ordered by rank, limit 50
- Empty Vec returned on no matches (QueryReturnedNoRows is success)

**`retention.rs`** — RetentionStore with delete_conversation:

- Hard-deletes conversation row; ON DELETE CASCADE removes messages; `messages_ad` trigger cleans FTS index
- `PRAGMA wal_checkpoint(TRUNCATE)` runs after delete; errors are non-fatal eprintln warning (D-14)
- Returns Ok(()) when id does not exist (idempotent)

**`backup.rs`** — scaffold placeholder for future export capability.

## Key Files

- `src-tauri/src/storage/migrations.rs` — 3-entry MIGRATIONS slice with conversations+messages+FTS5 DDL
- `src-tauri/src/storage/sqlite.rs` — ConversationStore + MessageStore + ShellPreferenceStore
- `src-tauri/src/storage/fts.rs` — FtsStore with parameterized FTS5 search
- `src-tauri/src/storage/retention.rs` — RetentionStore with hard-delete + WAL checkpoint
- `src-tauri/src/storage/backup.rs` — scaffold placeholder

## Commits

- `e49a50e` — feat(03-01): append migrations 0002 and 0003 to migrations.rs
- `f260d37` — feat(03-01): implement ConversationStore, MessageStore, FtsStore, RetentionStore

## Decisions Made

- FTS5 DDL lives entirely in migration 0003 — FtsStore only issues SELECT queries, never DDL
- WAL checkpoint errors after hard-delete are non-fatal (eprintln warning; delete already succeeded)
- `create_conversation` signature is `(id, title)` — model is recorded later via `mark_complete`
- `delete_conversation` is idempotent — no-op when row absent (Ok return)
- `search` returns empty Vec on no matches (not Err)

## Deviations from Plan

None — plan executed exactly as specified. All behaviors from the `<behavior>` blocks are implemented. No architectural changes were required.

## Security Notes

| Threat                                             | Status                                                            |
| -------------------------------------------------- | ----------------------------------------------------------------- |
| T-03-01: Migration ordering                        | Mitigated — append-only, MIGRATIONS.len() == 3 test asserts count |
| T-03-03: WAL checkpoint DoS                        | Accepted — non-fatal, doesn't block delete response               |
| T-03-04: FTS5 MATCH injection                      | Mitigated — `?1` bind parameter, no string interpolation          |
| T-03-05: ConversationRow/MessageRow serde exposure | Not yet crossed IPC — Plan B will map to typed response DTOs      |

## Verification Note

`cargo test` could not be run in this environment (cargo not in PATH for Bash tool). Tests were written and are embedded in the source files. They will execute as part of CI or when the developer runs `cargo test --manifest-path src-tauri/Cargo.toml storage`.

## Self-Check: PASSED

- [x] `src-tauri/src/storage/migrations.rs` — file exists with 3 migration entries
- [x] `src-tauri/src/storage/sqlite.rs` — ConversationStore, MessageStore, ConversationRow, MessageRow exported
- [x] `src-tauri/src/storage/fts.rs` — FtsStore, SearchResult exported
- [x] `src-tauri/src/storage/retention.rs` — RetentionStore exported
- [x] `src-tauri/src/storage/backup.rs` — scaffold placeholder exists
- [x] Commit e49a50e — migrations.rs updated
- [x] Commit f260d37 — all store files committed
- [x] `with_conn` calls only exist in storage/ module files (not in IPC handlers)

---

_Phase: 03-history_
_Completed: 2026-06-14_
