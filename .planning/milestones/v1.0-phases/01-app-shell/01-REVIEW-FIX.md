---
phase: 01-app-shell
fixed_at: 2026-06-13T00:00:00Z
review_path: .planning/phases/01-app-shell/01-REVIEW.md
iteration: 1
findings_in_scope: 8
fixed: 8
skipped: 0
status: all_fixed
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-06-13T00:00:00Z
**Source review:** .planning/phases/01-app-shell/01-REVIEW.md
**Iteration:** 1

**Summary:**

- Findings in scope: 8 (4 Critical, 4 Warning)
- Fixed: 8
- Skipped: 0

## Fixed Issues

### CR-01 + CR-02: Remove AppShell from layout, use Svelte 5 snippet pattern

**Files modified:** `src/routes/+layout.svelte`
**Commit:** db4f0a0
**Applied fix:** Rewrote `+layout.svelte` to remove the `<AppShell>` import and wrapper (which caused a double SurfaceRail in the DOM) and replaced the Svelte 4 `<slot />` with the Svelte 5 `{@render children?.()}` snippet pattern with a typed `Props` interface. The layout now solely handles hydration; `WorkspaceShell` in `+page.svelte` is the complete shell.

---

### CR-03: Eliminate race condition in `get_active_surface`

**Files modified:** `src-tauri/src/ipc/app_shell.rs`
**Commit:** fc8dc6a
**Applied fix:** Refactored `get_active_surface` to hold the shell mutex lock for the entire check-read-write hydration sequence instead of releasing it between the `hydrated` flag check and the DB read. Concurrent async invocations can no longer both observe `hydrated == false` and race to produce a stale return value from the `Ok(None)` branch. Added a doc comment documenting the lock ordering (shell -> sqlite).
**Note:** requires human verification — this is a concurrency logic fix.

---

### CR-04: Add migration ID validation and safety comment

**Files modified:** `src-tauri/src/storage/migrations.rs`
**Commit:** f1c2066
**Applied fix:** Added a `debug_assert!` before the `format!` call in `run_migrations` that validates the migration ID contains only ASCII alphanumeric characters and underscores. Added a `SAFETY` comment explaining that `migration.id` and `migration.sql` are `&'static str` literals and that this format pattern must not be generalized to dynamic values. Changed the format string to use explicit `\n` separators instead of embedded newlines.

---

### WR-01: Remove duplicate schema_migrations migration from MIGRATIONS slice

**Files modified:** `src-tauri/src/storage/migrations.rs`
**Commit:** 51410c0
**Applied fix:** Removed `Migration { id: "0001", description: "Create schema_migrations tracking table", ... }` from the `MIGRATIONS` slice. The inline bootstrap block in `run_migrations` is the correct and only place to create this table. Renumbered the former `0002` (shell_preferences) to `0001`. Updated the bootstrap comment to explain why schema_migrations is excluded from MIGRATIONS. Added a clarifying comment to the test asserting `applied == MIGRATIONS.len()`.

---

### WR-02: Propagate SQLite mutex poison as panic instead of InvalidParameterName

**Files modified:** `src-tauri/src/storage/sqlite.rs`
**Commit:** 8c51d48
**Applied fix:** Replaced the `map_err` that mapped mutex poison to `rusqlite::Error::InvalidParameterName` with `unwrap_or_else` that panics with a clear diagnostic message. Mutex poison after a thread panic is an unrecoverable state; the original code caused callers matching on `rusqlite::Error` to misclassify a fatal concurrency failure as a parameter validation failure and produce opaque `"InvalidParameterName: connection mutex poisoned"` messages in logs and IPC responses.

---

### WR-03: Update focusedIndex reactively after backend hydration

**Files modified:** `src/lib/components/SurfaceRail.svelte`
**Commit:** b34acc2
**Applied fix:** Replaced the IIFE initializer for `focusedIndex` (which evaluated `activeSurface` at construction time when it is always `'chat'`) with `let focusedIndex = $state(0)` and a `$effect` that keeps `focusedIndex` synchronized with `activeSurface`. This ensures the roving-tabindex pattern assigns `tabindex=0` to the correct button after `surfaceStore.hydrate()` resolves and updates the store's surface value.

---

### WR-04: Normalize IPC errors to user-facing strings in surface store

**Files modified:** `src/lib/stores/surface.ts`
**Commit:** 6b04320
**Applied fix:** Added a `normalizeIpcError(e: unknown): string` function that extracts the `message` field from serialized `ShellError` objects (`{ code, message }`), falls back to a human-readable `"Error: {code}"` string, or returns `"An unexpected error occurred."` for unknown shapes. Replaced both `error = String(e)` assignments in `hydrate()` and `setSurface()` with `error = normalizeIpcError(e)`. This prevents `"[object Object]"` from appearing verbatim in the status bar and aria-live region.

---

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-06-13T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
