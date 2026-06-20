---
phase: 01-single-agent-core
plan: "01"
subsystem: single-agent-core
tags:
  - phase-execution
  - single-agent-core
  - startup-recovery
  - turn-store
  - shell-hydration
dependency_graph:
  requires:
    - .planning/PROJECT.md
    - .planning/ROADMAP.md
    - .planning/STATE.md
    - .planning/phases/01-single-agent-core/01-CONTEXT.md
    - docs/architecture.md
    - docs/memory-loop.md
    - docs/implementation-plan.md
    - docs/agent-context.md
    - docs/privacy-boundaries.md
  provides:
    - authoritative conversation/turn/attempt boundary
    - fail-closed backend startup recovery
    - surface-only restart hydration contract
  affects:
    - src-tauri/src/storage/turns.rs
    - src-tauri/src/ipc/app_shell.rs
    - src-tauri/src/main.rs
    - src-tauri/tests/app_shell.rs
    - src/lib/stores/chat.ts
    - src/lib/stores/history.ts
    - src/lib/stores/surface.ts
    - src/routes/+layout.svelte
    - .planning/STATE.md
    - .planning/ROADMAP.md
tech-stack:
  added:
    - Rust
    - Tauri
    - Svelte 5
  patterns:
    - typed IPC
    - SQLite-backed shell persistence
    - restart hydration
    - fail-closed startup recovery
key-files:
  created:
    - .planning/phases/01-single-agent-core/01-SUMMARY.md
  modified:
    - src-tauri/src/storage/turns.rs
    - src-tauri/src/ipc/app_shell.rs
    - src-tauri/src/main.rs
    - src-tauri/tests/app_shell.rs
    - src/lib/stores/chat.ts
    - src/lib/stores/history.ts
    - src/lib/stores/surface.ts
    - src/routes/+layout.svelte
decisions:
  - Preserve the existing conversation/turn/attempt model as the Phase 1 contract.
  - Make startup recovery authoritative and fail closed on corruption or partial recovery.
  - Keep active conversation reopening user-driven; restart restores shell surface only.
metrics:
  duration: about 1h 30m
  completed_date: 2026-06-20
status: complete
---

# Phase 1 Plan 01: Single-Agent Core Summary

Single-agent core now uses the existing conversation/turn/attempt contract as the durable phase boundary. Startup recovery is authoritative, shell hydration no longer masks backend storage errors, and restart behavior restores the last active surface without auto-reopening conversation state.

## What Changed

- Hardened orphan recovery in `src-tauri/src/storage/turns.rs` so partial recovery aborts instead of silently proceeding.
- Made shell hydration in `src-tauri/src/ipc/app_shell.rs` propagate storage failures and leave the shell unready on corruption.
- Moved startup recovery in `src-tauri/src/main.rs` to a fail-closed path before renderer-facing state is exposed.
- Updated the Svelte shell hydration trigger and related stores so the UI treats backend readiness as authoritative while conversation reopening remains user-driven.
- Added restart and corruption regressions in `src-tauri/tests/app_shell.rs` and backend unit tests.

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml --test app_shell`
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`
- `npm run check`
- `git diff --check`

All four checks passed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Shell hydration was masking backend storage failures**
- **Found during:** Task 2
- **Issue:** `get_active_surface_inner` ignored SQLite load errors and still marked the shell hydrated, which could present a ready shell after a broken startup path.
- **Fix:** Propagated the storage error, left hydration false on failure, and added restart/corruption regressions.
- **Files modified:** `src-tauri/src/ipc/app_shell.rs`, `src-tauri/tests/app_shell.rs`
- **Commit:** `e9847f6`

**2. [Rule 1 - Bug] Orphan recovery accepted partial/corrupted parent chains**
- **Found during:** Task 1
- **Issue:** `recover_orphaned_attempts` could succeed even when in-flight attempts were detached from a valid turn/conversation chain.
- **Fix:** Added pre/post recovery verification and fail-closed behavior on partial recovery.
- **Files modified:** `src-tauri/src/storage/turns.rs`
- **Commit:** `f00bc50`

## Self-Check

PASSED

- Created file exists: `.planning/phases/01-single-agent-core/01-SUMMARY.md`
- Task commit exists: `f00bc50`
- Task commit exists: `e9847f6`
- Rust verification passed
- Frontend verification passed
- Diff sanity check passed

