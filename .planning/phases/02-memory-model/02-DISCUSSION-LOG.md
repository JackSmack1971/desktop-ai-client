# Phase 2: Memory Model - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-21
**Phase:** 2-memory-model
**Areas discussed:** Phase scope, Tag source, Inspection surface

---

## Phase scope

Codebase scout found `src-tauri/src/storage/memory.rs` (migration 0006) already implements most of Phase 2's success criteria, predating this roadmap's phase numbering. Two criteria remain unmet: candidates have no `tags` field, and there is no documented way to inspect memory metadata.

| Option | Description | Selected |
|--------|-------------|----------|
| Both gaps | Add tags column + inspection surface | ✓ |
| Tags only | Add tags; defer inspection | |
| Inspection only | Add inspection surface; defer tags | |
| Something else | Redefine Phase 2 scope | |

**User's choice:** Both gaps
**Notes:** Confirms the other three roadmap success criteria (separate storage, candidate metadata minus tags, shadow-mode isolation) are already satisfied and need no new work.

---

## Tag source

| Option | Description | Selected |
|--------|-------------|----------|
| Free-form, caller-supplied | `Vec<String>` chosen by the proposing code, no taxonomy | ✓ |
| Fixed taxonomy | Closed enum/set, validated at insert | |
| Derived automatically | Inferred from kind/source context | |

**User's choice:** Free-form, caller-supplied
**Notes:** Simplest option, no taxonomy to maintain. Fixed taxonomy noted as a future option if free-form tags prove unstructured.

---

## Inspection surface

| Option | Description | Selected |
|--------|-------------|----------|
| CLI binary | Extend memory-replay binary pattern | |
| Tauri IPC command | Typed IPC command + Svelte debug view | |
| Direct SQLite read | Document read-only query pattern, no new code | ✓ |

**User's choice:** Direct SQLite read
**Notes:** No new Rust code, no IPC, no UI. Preserves the current shadow-mode isolation (confirmed via codebase scout: no `memory::` references exist anywhere in `src-tauri/src/ipc/`). Query pattern must avoid joins/columns that would expose secrets or raw file paths from other tables.

---

## Claude's Discretion

- Exact tags column encoding (JSON array vs delimited TEXT) and new migration numbering.
- Where the inspection doc lives (update `docs/memory-loop.md` vs a new file).
- Whether `propose_candidate`'s tags parameter is required or optional/defaulted, and how existing call sites are updated.

## Deferred Ideas

- Tauri IPC command + Svelte debug UI for memory inspection — revisit if/when shadow mode graduates toward live behavior.
- Fixed tag taxonomy — revisit if free-form tags prove too unstructured (e.g., during Phase 5 consolidation work).
