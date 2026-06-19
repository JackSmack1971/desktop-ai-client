# Phase 3: History - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 03-history
**Areas discussed:** Conversation schema, History surface UX, Search design, Retention & deletion

---

## Conversation Schema

| Option                                   | Description                                                                            | Selected |
| ---------------------------------------- | -------------------------------------------------------------------------------------- | -------- |
| Two tables (conversations + messages FK) | Cleanest — separate queries for list vs. messages, retention targets conversations row | ✓        |
| One table with grouping                  | No conversations table; metadata denormalized per row or derived at query time         |          |

**User's choice:** Two tables

---

| Option                                 | Description                                                              | Selected |
| -------------------------------------- | ------------------------------------------------------------------------ | -------- |
| Auto-generated from first user message | Backend truncates first user message to ~60 chars; no user action needed | ✓        |
| Timestamp-based default                | Title = "Chat on 2026-06-14 at 2:00 PM"                                  |          |
| You decide                             | Planner picks                                                            |          |

**User's choice:** Auto-generated from first user message

---

| Option           | Description                                                         | Selected |
| ---------------- | ------------------------------------------------------------------- | -------- |
| Full metadata    | id, title, model, status, message_count, created_at, updated_at     |          |
| Pragmatic middle | id, title, model, status, created_at, updated_at — no message_count | ✓        |
| Minimal metadata | id, title, status, created_at, updated_at — drop model              |          |
| You decide       | Planner picks fields                                                |          |

**User's choice:** Pragmatic middle — store model + status; drop message_count (derive via COUNT)
**Notes:** model is high-value (comes for free from Phase 2's ChatEvent::Done), status needed for retention/UI, message_count cheap to derive via the index.

---

| Option                          | Description                                                                           | Selected |
| ------------------------------- | ------------------------------------------------------------------------------------- | -------- |
| Verbatim                        | Full message content in SQLite; redaction applies to logs/telemetry not local history | ✓        |
| Redaction pipeline before write | Run through security::redaction before storing                                        |          |

**User's choice:** Verbatim
**Notes:** Privacy boundary is IPC layer + typed backend commands, not the storage layer itself. Redaction applies to egress surfaces (logs, telemetry, crash reports, exports, support bundles). Full-fidelity content required for FTS5 (HIST-02) and future artifact reconstruction.

---

## History Surface UX

| Option                     | Description                                                              | Selected |
| -------------------------- | ------------------------------------------------------------------------ | -------- |
| Conversation list          | Scrollable list, search bar at top, title/model/status/timestamp per row | ✓        |
| Timeline / grouped by date | Conversations under "Today", "Yesterday", etc.                           |          |
| You decide                 | Planner determines layout                                                |          |

**User's choice:** Simple scrollable conversation list with search bar at the top
**Notes:** Matches minimal, focused UI direction (Claude/ChatGPT-style). Keeps HistorySurface small and accessible.

---

| Option                         | Description                                                          | Selected |
| ------------------------------ | -------------------------------------------------------------------- | -------- |
| Load into Chat surface         | Navigate to Chat with conversation_id; user can continue immediately | ✓        |
| In-place read-only detail view | Expand inline; separate Continue button to load into Chat            |          |

**User's choice:** Load into Chat surface

---

| Option                                | Description                                                         | Selected |
| ------------------------------------- | ------------------------------------------------------------------- | -------- |
| Full message history, input ready     | All messages rendered, input pre-focused, no banner                 | ✓        |
| Full history with continuation banner | Messages + "Continuing conversation from June 12 • model" separator |          |

**User's choice:** Full message history, input ready — no continuation banner
**Notes:** Thread context + conversation title/model in header is sufficient. Standard pattern.

---

| Option                                  | Description                                                   | Selected |
| --------------------------------------- | ------------------------------------------------------------- | -------- |
| Dedicated "New conversation" button     | "New chat" in Chat header or nav rail; explicit, discoverable | ✓        |
| Navigating to Chat with no conversation | Navigation acts as new chat trigger                           |          |

**User's choice:** Dedicated "New chat" button
**Notes:** Standard, discoverable pattern used by every major chat app.

---

## Search Design

| Option                                | Description                                             | Selected |
| ------------------------------------- | ------------------------------------------------------- | -------- |
| Message content only                  | External-content FTS5 on messages.content with triggers | ✓        |
| Message content + conversation titles | Index both; more coverage, more complex FTS5 setup      |          |

**User's choice:** Message content only
**Notes:** Spec alignment — adversarial v5 architecture describes FTS5 on messages table with triggers. Adding titles would require denormalization or a second FTS table. Users search for what was discussed, not auto-generated titles.

---

| Option                       | Description                       | Selected |
| ---------------------------- | --------------------------------- | -------- |
| Debounced real-time (~300ms) | Results update live as user types | ✓        |
| Submit-to-search             | User presses Enter to trigger     |          |

**User's choice:** Debounced real-time

---

| Option                     | Description                                                 | Selected |
| -------------------------- | ----------------------------------------------------------- | -------- |
| Conversation row + snippet | Title/model/status/timestamp + ~80 char highlighted snippet | ✓        |
| Conversation row only      | Title and metadata, no snippet                              |          |
| Individual message rows    | Each matching message gets its own row                      |          |

**User's choice:** Conversation-level result with highlighted snippet from matching message content

---

## Retention & Deletion

| Option                      | Description                                                       | Selected |
| --------------------------- | ----------------------------------------------------------------- | -------- |
| Manual delete only          | Explicit user-initiated delete; no auto-expiry in Phase 3         | ✓        |
| Manual delete + auto-expiry | Auto-delete conversations older than N days via hardcoded default |          |
| You decide                  | Planner scopes                                                    |          |

**User's choice:** Manual delete only for Phase 3

---

| Option                  | Description                                              | Selected |
| ----------------------- | -------------------------------------------------------- | -------- |
| Hard delete             | DELETE + CASCADE; FTS5 triggers handle index cleanup     | ✓        |
| Soft delete (tombstone) | deleted_at timestamp; queries filter; cleanup job needed |          |

**User's choice:** Hard delete

---

| Option                                     | Description                                                  | Selected |
| ------------------------------------------ | ------------------------------------------------------------ | -------- |
| Out of scope — leave backup.rs as scaffold | WAL handled automatically by SQLite                          |          |
| In scope — WAL checkpoint after delete     | PRAGMA wal_checkpoint(TRUNCATE) triggered after hard deletes | ✓        |

**User's choice:** In scope — implement WAL checkpoint after delete
**Notes:** Adversarial v5 spec explicitly flags WAL lifecycle as a hardening item: "No unmanaged WAL state." Destructive operations must account for checkpoints, busy readers, and disk space reclamation.

---

## Claude's Discretion

- Exact FTS5 tokenizer configuration (unicode61 vs. ascii)
- SQL index selection beyond the FK index on messages.conversation_id
- Exact IPC command names for history\_\* surface
- WAL checkpoint error handling on busy readers (non-fatal warning)

## Deferred Ideas

- Auto-expiry / scheduled retention (future settings phase)
- Soft delete / undo with recovery window (not needed with hard delete + WAL)
- Conversation renaming (future phase — needs settings or inline edit UX)
- Export / import backup (backup.rs scaffold left for this)
- Title search in FTS5 (can be added if users need it)
