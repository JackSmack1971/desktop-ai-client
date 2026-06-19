---
phase: 01-app-shell
reviewed: 2026-06-13T00:00:00Z
depth: standard
files_reviewed: 27
files_reviewed_list:
  - src-tauri/Cargo.toml
  - src-tauri/build.rs
  - src-tauri/capabilities/main.json
  - src-tauri/src/app_state.rs
  - src-tauri/src/ipc/app_shell.rs
  - src-tauri/src/ipc/mod.rs
  - src-tauri/src/lib.rs
  - src-tauri/src/main.rs
  - src-tauri/src/storage/migrations.rs
  - src-tauri/src/storage/mod.rs
  - src-tauri/src/storage/sqlite.rs
  - src-tauri/tauri.conf.json
  - src-tauri/tests/app_shell.rs
  - src-tauri/permissions/app-shell.toml
  - src/app.html
  - src/lib/components/AppShell.svelte
  - src/lib/components/StatusRegion.svelte
  - src/lib/components/SurfacePanel.svelte
  - src/lib/components/SurfaceRail.svelte
  - src/lib/components/WorkspaceShell.svelte
  - src/lib/components/surfaces/ArtifactsSurface.svelte
  - src/lib/components/surfaces/ChatSurface.svelte
  - src/lib/components/surfaces/HistorySurface.svelte
  - src/lib/components/surfaces/SettingsSurface.svelte
  - src/lib/stores/surface.ts
  - src/routes/+layout.svelte
  - src/routes/+page.svelte
  - tests/rust/app_shell.rs
findings:
  critical: 4
  warning: 4
  info: 2
  total: 10
status: issues_found
---

# Phase 01: App Shell — Code Review Report

**Reviewed:** 2026-06-13T00:00:00Z
**Depth:** standard
**Files Reviewed:** 27
**Status:** issues_found

## Summary

This phase delivers the backend-owned surface-preference persistence layer (Rust/Tauri/SQLite) and the Svelte 5 workspace shell frontend. The core architecture is sound: surface state lives in the backend, IPC is capability-gated with both a permission file and backend-side window-label enforcement, no browser storage is used, and migrations run inside `SqlitePool::open` before the first command handler executes.

Four blockers were identified:

1. The Svelte layout renders `AppShell` which duplicates the navigation rail already provided by `WorkspaceShell` — the rail renders twice in the live DOM.
2. `+layout.svelte` uses the Svelte 4 `<slot />` syntax while the rest of the codebase uses the Svelte 5 `{@render children?.()}` snippet pattern — page content will not project through the layout in strict Svelte 5 mode.
3. `get_active_surface` has a race condition: concurrent async invocations can both pass the `!hydrated` guard, causing duplicate DB reads and a stale-value return path.
4. The migration savepoint name is assembled with `format!` string interpolation of migration IDs and SQL, establishing a precedent that will become an injection vector if any future migration introduces dynamic content.

---

## Critical Issues

### CR-01: Double `SurfaceRail` render — `AppShell` in layout wraps `WorkspaceShell` in page

**File:** `src/routes/+layout.svelte:14` and `src/routes/+page.svelte:22`

**Issue:** `+layout.svelte` renders `<AppShell>` which unconditionally renders `<SurfaceRail>` inside a `<nav aria-label="Surface navigation">`. `+page.svelte` renders `<WorkspaceShell>` — also unconditionally rendering `<SurfaceRail>` inside its own `<nav aria-label="Surface navigation">`. Because the SvelteKit layout wraps the page, both `AppShell` and `WorkspaceShell` are live in the DOM simultaneously. The result is:

- Two navigation rails visible on screen, each with four buttons.
- Two independent `focusedIndex` state variables in two `SurfaceRail` component instances. Keyboard arrow-key navigation in one rail has no effect on the other, so focus management is split.
- Two `role="tablist"` landmarks with the same `aria-label="Workspace surfaces"` — invalid ARIA, screen readers will announce duplicate navigation regions.
- Both rails call `surfaceStore.setSurface()` on their respective `handleClick`, so the singleton store receives correct updates, but the two `focusedIndex` states diverge.
- `AppShell` and `WorkspaceShell` both render a root element with `aria-label="Desktop AI Client"`, producing nested duplicate application labels.

**Fix:** Remove `<AppShell>` from `+layout.svelte`. The layout's sole responsibility is hydration. `WorkspaceShell` in `+page.svelte` is the complete shell component and should not be wrapped by `AppShell`:

```svelte
<!-- src/routes/+layout.svelte — corrected -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { surfaceStore } from '$lib/stores/surface';

	interface Props {
		children?: import('svelte').Snippet;
	}
	let { children }: Props = $props();

	onMount(() => {
		surfaceStore
			.hydrate()
			.catch((e) => console.error('surface hydration failed', e));
	});
</script>

{@render children?.()}
```

`AppShell.svelte` should then be deleted or explicitly marked as superseded by `WorkspaceShell.svelte`.

---

### CR-02: Svelte 4 `<slot />` in `+layout.svelte` — page content will not render in Svelte 5 strict mode

**File:** `src/routes/+layout.svelte:16`

**Issue:** `+layout.svelte` uses `<slot />` (the Svelte 4 children-projection API). Every other component in the codebase — `AppShell.svelte`, `SurfacePanel.svelte`, `WorkspaceShell.svelte` — uses the Svelte 5 `{@render children?.()}` snippet pattern with `interface Props { children?: import('svelte').Snippet; }`. Mixing these APIs in the same tree is undefined in Svelte 5: the `<slot />` element is treated as an unknown HTML element (rendered as a literal `<slot>` tag) and the children snippet is never rendered. The practical result is a blank page body — the page route content never appears.

**Fix:** Update `+layout.svelte` to use the Svelte 5 snippet pattern (as shown in CR-01 fix above). This also aligns with the fix for CR-01 by removing the `AppShell` import entirely.

---

### CR-03: Race condition in `get_active_surface` — concurrent calls both pass the `!hydrated` guard

**File:** `src-tauri/src/ipc/app_shell.rs:44-70`

**Issue:** The function reads `(current, hydrated)` under one lock acquisition (lines 44-49) then releases the lock. If two concurrent async invocations of `get_active_surface` arrive on the Tokio multi-thread runtime before either completes, both observe `hydrated == false` at their respective first lock acquisitions. Both then call `store.load_active_surface()` independently and race to re-acquire the `AppState::shell` mutex. The concrete failure mode:

1. Both callers read `hydrated = false` and `current = Chat` (default).
2. Both call `store.load_active_surface()`, getting back `Some(History)` (hypothetical stored value).
3. Both enter the `Ok(Some(persisted))` branch.
4. Caller A acquires the lock, writes `active_surface = History`, sets `hydrated = true`, returns `Ok(History)`.
5. Caller B acquires the lock, writes `active_surface = History` again (idempotent for this branch), returns `Ok(History)`.

The returned values happen to be correct, but the extra DB read is wasted and — more critically — in the `Ok(None)` branch (no persisted value) the two callers both reach line 64 to set `hydrated = true`. There, `current` was captured as the pre-hydration default. If caller A already completed and `set_active_surface` wrote a value between caller B's first lock release and its second lock acquisition, caller B still returns the stale `current` captured before hydration. The stale return is silent.

**Fix:** Hold the lock for the full hydration sequence — check, DB read, and write — without releasing it in between. Since `store.load_active_surface()` takes a separate internal mutex (`SqlitePool::conn`), document the lock-ordering (shell → sqlite) and ensure no code path acquires these in the opposite order:

```rust
pub async fn get_active_surface(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, ShellPreferenceStore>,
) -> Result<Surface, ShellError> {
    assert_main_window(&window)?;

    let mut shell = state.shell.lock().map_err(|e| {
        ShellError::StorageError(format!("shell state lock poisoned: {e}"))
    })?;

    if !shell.hydrated {
        // DB read while holding the shell lock; lock ordering: shell -> sqlite.
        if let Ok(Some(persisted)) = store.load_active_surface() {
            shell.active_surface = persisted;
        }
        shell.hydrated = true;
    }

    Ok(shell.active_surface.clone())
}
```

Note: `store.load_active_surface()` calls `SqlitePool::with_conn` which acquires `SqlitePool::conn`'s mutex. The ordering (shell lock held, then sqlite lock acquired) must be consistently maintained across all callers to prevent deadlock.

---

### CR-04: SQL injection precedent — migration savepoint name and SQL body assembled via `format!`

**File:** `src-tauri/src/storage/migrations.rs:87-93`

**Issue:** The savepoint name and migration SQL are embedded directly into a format string passed to `conn.execute_batch`:

```rust
let result = conn.execute_batch(&format!(
    "SAVEPOINT migration_{id};
     {sql}
     RELEASE SAVEPOINT migration_{id};",
    id = migration.id,
    sql = migration.sql,
));
```

`migration.id` and `migration.sql` are `&'static str` constants today, so this cannot be exploited at runtime in the current codebase. The concern is structural:

1. **Injection precedent**: The pattern teaches future contributors that string interpolation into raw SQL is acceptable. When future migrations add more dynamic content (e.g., table names derived from configuration), this pattern will be copied and become exploitable.
2. **Savepoint name injection**: SQLite savepoint names must be valid SQL identifiers. If any future migration ID contains `'`, `;`, whitespace, or `--`, the batch would be malformed. The IDs are validated only by convention, not by code.
3. **SQL body interpolation**: `{sql}` is interpolated with no escaping. If any future migration constructs its SQL from external input (e.g., a plugin name from a config file), this is a direct injection path.

**Fix:** Add an assertion to validate migration IDs before use, and add a code comment explaining why SQL interpolation is acceptable here (static constants only) and must never be extended to dynamic values:

```rust
// Validate id is a safe SQL identifier before embedding it.
debug_assert!(
    migration.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
    "migration id must be alphanumeric/underscore only: {:?}",
    migration.id
);
// SAFETY: migration.id and migration.sql are &'static str literals defined
// in MIGRATIONS. Do not generalize this pattern to dynamic values.
let result = conn.execute_batch(&format!(
    "SAVEPOINT migration_{id};\n{sql}\nRELEASE SAVEPOINT migration_{id};",
    id = migration.id,
    sql = migration.sql,
));
```

---

## Warnings

### WR-01: Migration 0001 (`schema_migrations`) duplicated — bootstrap inline and in `MIGRATIONS` slice

**File:** `src-tauri/src/storage/migrations.rs:28-41` and `60-69`

**Issue:** `run_migrations` begins with an inline `CREATE TABLE IF NOT EXISTS schema_migrations` (lines 60-69) to bootstrap the tracking table. Migration `0001` in the `MIGRATIONS` array (lines 28-41) repeats the same DDL with `IF NOT EXISTS`. On first run: the inline creates the table, then the loop reaches `0001`, attempts the same DDL (succeeds as a no-op), marks it as applied (`success = 1`), and increments `applied`. The function reports `applied = 2` for a fresh database, which the test at line 130 asserts against `MIGRATIONS.len()` — correct, but misleading. Migration `0001` did not meaningfully advance the schema; the table was already present from the inline bootstrap.

More importantly: if migration `0001` is ever marked `success = 0` (e.g., a failed run wrote a bad row), the repair path is confusing — the table exists (created by the inline bootstrap) but `0001` reports failure. A developer diagnosing the incident must understand the dual-creation path to avoid concluding the tracking table is corrupt.

**Fix:** Remove `Migration { id: "0001", ... }` from `MIGRATIONS`. The inline bootstrap block is the correct and only place to create `schema_migrations`. Renumber the existing `0002` to `0001`. Update the test to assert `applied == 1` (or `MIGRATIONS.len()`) and add a comment explaining why the tracking table is bootstrapped outside the migration loop.

---

### WR-02: `SqlitePool::with_conn` maps mutex-poison to `rusqlite::Error::InvalidParameterName`

**File:** `src-tauri/src/storage/sqlite.rs:61-64`

**Issue:**

```rust
let conn = self.conn.lock().map_err(|_| {
    rusqlite::Error::InvalidParameterName("connection mutex poisoned".to_string())
})?;
```

`rusqlite::Error::InvalidParameterName` is semantically a parameter-binding validation error (wrong column count, wrong type). Using it to represent a mutex-poison condition causes callers that match on `rusqlite::Error` to misclassify a fatal concurrency failure as an input validation failure. Call sites in `ShellPreferenceStore::save_active_surface` and `load_active_surface` propagate this up as `ShellError::StorageError("InvalidParameterName: connection mutex poisoned")`, which is an opaque and misleading diagnostic in logs and IPC responses.

**Fix:** Mutex poison after a thread panic is an unrecoverable state. Propagate it as a panic or define a wrapper error:

```rust
let conn = self.conn.lock().unwrap_or_else(|poisoned| {
    // A prior thread panicked while holding the connection lock.
    // The connection state is unknown; propagate the panic.
    panic!("SQLite connection mutex poisoned: {}", poisoned);
});
```

If recovery is required, define a thin error type that can carry both `rusqlite::Error` and `MutexPoisoned` so call sites can match correctly.

---

### WR-03: `focusedIndex` initializer in `SurfaceRail` never updates after backend hydration changes `activeSurface`

**File:** `src/lib/components/SurfaceRail.svelte:31-33`

**Issue:** `focusedIndex` is initialized once at component mount from `activeSurface`:

```ts
let focusedIndex = $state(
	(() => {
		const idx = surfaces.findIndex((s) => s.id === activeSurface);
		return idx === -1 ? 0 : idx;
	})(),
);
```

`activeSurface` is `$derived(surfaceStore.surface)`, which is `'chat'` (the default) at component construction time. The `surfaceStore.hydrate()` call in `+layout.svelte`'s `onMount` fires after the component tree is already mounted. When hydration completes and updates `surfaceStore.surface` to, say, `'history'`, `activeSurface` re-derives correctly (so the active-highlight on the buttons updates), but `focusedIndex` is never updated — it remains `0` (Chat). The roving-tabindex pattern gives `tabindex=0` to the Chat button while the History button appears highlighted as active. Keyboard Tab navigation therefore starts on the wrong button.

**Fix:** Update `focusedIndex` reactively when `activeSurface` changes, using `$effect`:

```ts
$effect(() => {
	const idx = surfaces.findIndex((s) => s.id === activeSurface);
	if (idx !== -1) focusedIndex = idx;
});
```

This also fixes the IIFE initialization — replace the IIFE with `let focusedIndex = $state(0);` and let the effect set the correct initial value on first run.

---

### WR-04: `surfaceStore.error` exposes raw IPC error objects rendered verbatim in the status bar

**File:** `src/lib/stores/surface.ts:45` and `src/lib/components/WorkspaceShell.svelte:65`

**Issue:** When a backend IPC call fails, the error is stored as `error = String(e)`. The Tauri IPC layer rejects errors as serialized `ShellError` objects: `{ code: "STORAGE_ERROR", message: "..." }`. `String({ code: "STORAGE_ERROR", message: "..." })` in JavaScript produces `"[object Object]"` — a useless string that is then displayed verbatim in the `StatusRegion` status bar and announced via `aria-live`. When the error is a plain string (e.g., a permission rejection), it is rendered verbatim and may include Tauri-internal error identifiers that mean nothing to the user.

**Fix:** Normalize IPC errors to user-facing strings at the store boundary:

```typescript
function normalizeIpcError(e: unknown): string {
	if (typeof e === 'string') return e;
	if (e && typeof e === 'object') {
		const obj = e as Record<string, unknown>;
		if (typeof obj['message'] === 'string') return obj['message'];
		if (typeof obj['code'] === 'string') return `Error: ${obj['code']}`;
	}
	return 'An unexpected error occurred.';
}

// Replace:  error = String(e);
// With:     error = normalizeIpcError(e);
```

---

## Info

### IN-01: `AppShell.svelte` is a dead/superseded component

**File:** `src/lib/components/AppShell.svelte:1-63`

**Issue:** `AppShell.svelte` provides a side-rail-plus-content-slot layout that is a strict subset of what `WorkspaceShell.svelte` delivers. Once CR-01 is resolved (removing `AppShell` from the layout), `AppShell.svelte` will have no callers. Leaving dead components in the tree causes confusion: future contributors may reach for `AppShell` instead of `WorkspaceShell`, reintroducing the double-rail bug.

**Fix:** Delete `src/lib/components/AppShell.svelte` after resolving CR-01.

---

### IN-02: `SURFACE_LABELS` mapping is duplicated across three frontend files

**File:** `src/lib/stores/surface.ts:18-23`, `src/lib/components/WorkspaceShell.svelte:33-38`, `src/routes/+page.svelte:10-15`

**Issue:** The `Surface -> string` label mapping is defined identically in three places. Adding a new surface requires updating all three. Currently all three agree; the duplication is a future maintenance hazard rather than a current bug.

**Fix:** Export `SURFACE_LABELS` from `surface.ts` (where the canonical type is defined) and import it in the other two files:

```typescript
// surface.ts — add export keyword
export const SURFACE_LABELS: Record<Surface, string> = {
	chat: 'Chat',
	history: 'History',
	settings: 'Settings',
	artifacts: 'Artifacts',
};
```

```svelte
<!-- WorkspaceShell.svelte and +page.svelte — remove local definition -->
import { surfaceStore, type Surface, SURFACE_LABELS } from '$lib/stores/surface';
```

---

_Reviewed: 2026-06-13T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
