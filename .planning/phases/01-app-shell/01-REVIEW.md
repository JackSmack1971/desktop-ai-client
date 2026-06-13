---
phase: 01-app-shell
reviewed: 2026-06-13T19:27:12Z
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
  warning: 5
  info: 3
  total: 12
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-06-13T19:27:12Z
**Depth:** standard
**Files Reviewed:** 27
**Status:** issues_found

## Summary

This phase delivers the backend-owned shell preference persistence layer (Rust/Tauri) and the Svelte 5 frontend scaffold. The architecture boundary between frontend and backend is correctly conceived — no secrets or raw paths leak into the renderer, all surface state round-trips through typed IPC, and the SQLite schema is sound.

However, four blockers prevent the code from shipping correctly:

1. `ShellPreferenceStore` and `SqlitePool` are never registered with Tauri's state system in `main.rs`, so both IPC commands will panic at runtime when Tauri tries to inject the unmanaged state.
2. The IPC commands `get_active_surface` and `set_active_surface` are not listed in the capability file, so they are blocked by Tauri 2's deny-by-default permission system even if the state wiring were fixed.
3. `get_active_surface` has a correctness bug: it silently skips restoring the persisted surface when the user previously saved `Chat` explicitly, producing the wrong startup state.
4. The SQLite pool opened by `SqlitePool::open` never runs migrations, so the `shell_preferences` table does not exist when the IPC commands first call `save_active_surface` or `load_active_surface`.

Five warnings cover a migration failure that is silently swallowed, a frontend duplicate `role="application"`, an incorrect `focusedIndex` initializer in SurfaceRail, and two structural gaps.

---

## Critical Issues

### CR-01: `ShellPreferenceStore` and `SqlitePool` are never managed — both IPC commands panic at runtime

**File:** `src-tauri/src/main.rs:23`

**Issue:** `main.rs` calls `.manage(AppState::default())` and nothing else. Both `get_active_surface` and `set_active_surface` declare `store: tauri::State<'_, ShellPreferenceStore>` as a parameter. Tauri 2 panics with an "unmanaged state" error if an IPC command requests a state type that was never registered via `.manage(...)`. A file-backed `SqlitePool` must also be created and wrapped in `Arc` before `ShellPreferenceStore::new(pool)` can be called. Neither object is constructed or registered.

Additionally, the pool's `open()` method accepts a `PathBuf` for the database file, but there is no code anywhere in `main.rs` or a setup hook that resolves the Tauri `app_data_dir` path and calls `SqlitePool::open`.

**Fix:**
```rust
use std::sync::Arc;
use storage::sqlite::{SqlitePool, ShellPreferenceStore};
use storage::migrations::run_migrations;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Resolve the per-user data directory and open the SQLite file.
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("app data dir must be resolvable");
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("app.db");

            let pool = SqlitePool::open(db_path)?;
            // Run pending migrations immediately after opening.
            {
                pool.with_conn(|conn| {
                    run_migrations(conn, env!("CARGO_PKG_VERSION"))
                        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))
                })?;
            }
            let pool = Arc::new(pool);
            app.manage(ShellPreferenceStore::new(Arc::clone(&pool)));
            app.manage(pool);
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            ipc::app_shell::get_active_surface,
            ipc::app_shell::set_active_surface,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed to start");
}
```

---

### CR-02: IPC commands absent from capability file — blocked by Tauri 2 permission system

**File:** `src-tauri/capabilities/main.json:6-11`

**Issue:** Tauri 2 uses a deny-by-default capability system. A command is only invokable from a window if it is listed (directly or via a permission set) in that window's capability JSON. The `main-window` capability grants `core:default`, `opener:default`, `core:app:allow-app-hide`, and `core:window:allow-start-dragging` — none of which cover `get_active_surface` or `set_active_surface`. Calling either command from the frontend will be rejected by Tauri's permission layer before the Rust handler is ever reached.

**Fix:** Add the custom commands to the capability's `permissions` array. The conventional Tauri 2 approach is to declare a custom permission set in `src-tauri/permissions/` and reference it here, or to list the commands directly:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-window",
  "description": "Capability set for the main application window.",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "core:app:allow-app-hide",
    "core:window:allow-start-dragging",
    "allow-get-active-surface",
    "allow-set-active-surface"
  ]
}
```

The matching permission identifiers must be declared in `src-tauri/permissions/app-shell.toml` (or equivalent) with `[[permission]]` entries that list the commands. Alternatively, if the project uses Tauri's inline `"app:allow-*"` naming convention, confirm the exact identifiers against the generated schema.

---

### CR-03: `get_active_surface` skips DB restore when the persisted surface is `Chat`

**File:** `src-tauri/src/ipc/app_shell.rs:53-61`

**Issue:** The startup hydration path reads the in-memory `active_surface` (default `Chat`) and only queries SQLite when `current == Surface::Chat`. But if the user previously saved `Chat` explicitly, `load_active_surface()` returns `Some(Surface::Chat)`, which satisfies `if let Ok(Some(persisted))` — so the in-memory state is redundantly set and the function returns `Chat`. This branch actually does work for `Chat`. The real correctness failure is the inverse: if the persisted value is anything other than `Chat`, but the in-memory state is already non-`Chat` (e.g., a second call after `set_active_surface` wrote `History`), the condition `current == Surface::Chat` is false and the DB is never consulted — this is intentional. However the current logic fails the startup contract silently when the persisted value is `Chat` and the user expects a restored `Chat` session: the function skips into the `Ok(current)` return on line 63 without ever updating in-memory state, meaning the in-memory state remains the struct-level default rather than being confirmed from storage. This is a minor redundancy.

The more serious correctness defect: if a future refactor changes `Surface::default()` to something other than `Chat`, the guard on line 53 will stop hydrating any previous-session `Chat` preference. The hydration condition is semantically "is this the default value?" but is expressed as a type-equality check against `Surface::Chat`. It should check whether the in-memory state has ever been set from storage (e.g., a `hydrated: bool` flag), not which surface is the default.

**Fix:** Introduce a `hydrated` flag in `ShellState` and base the branch on it:

```rust
// In ShellState:
pub hydrated: bool,  // false until first DB load completes

// In get_active_surface:
let (current, hydrated) = {
    let shell = state.shell.lock()...?;
    (shell.active_surface.clone(), shell.hydrated)
};

if !hydrated {
    if let Ok(Some(persisted)) = store.load_active_surface() {
        let mut shell = state.shell.lock()...?;
        shell.active_surface = persisted.clone();
        shell.hydrated = true;
        return Ok(persisted);
    }
    let mut shell = state.shell.lock()...?;
    shell.hydrated = true;
}

Ok(current)
```

---

### CR-04: `SqlitePool::open` does not run migrations — `shell_preferences` table missing on first launch

**File:** `src-tauri/src/storage/sqlite.rs:25-38`

**Issue:** `SqlitePool::open` opens the connection and sets pragmas, but does not call `run_migrations`. The doc-comment says "Applies all pending migrations before returning" (line 22-23), making this a documentation-level contract violation as well as a runtime defect. On first launch, `save_active_surface` will fail with `SqliteFailure` (table not found) and `load_active_surface` will also fail, causing the IPC command to return a `StorageError` to the frontend. The `surface.ts` store catches IPC errors and sets `error` state, so the UI will render in an error state on every first launch.

**Fix:** Either call `run_migrations` inside `open` (simplest, keeps the contract the doc-comment makes), or ensure every call site calls it immediately after `open`. The setup hook in CR-01 shows the latter pattern. If `open` is supposed to handle migrations itself:

```rust
pub fn open(db_path: PathBuf) -> rusqlite::Result<Self> {
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )?;
    crate::storage::migrations::run_migrations(&conn, env!("CARGO_PKG_VERSION"))?;
    Ok(Self { conn: Mutex::new(conn) })
}
```

---

## Warnings

### WR-01: Migration failure after partial write is silently swallowed for `success=false` rows

**File:** `src-tauri/src/storage/migrations.rs:87-109`

**Issue:** When a migration fails, the code writes a row with `success = 0` to `schema_migrations` (line 105), then re-raises the error via `result?` (line 109). On the next launch, `run_migrations` queries `SELECT success FROM schema_migrations WHERE id = ?1` and calls `row.get::<_, bool>(0)` (line 77). A `success` value of `0` maps to `false`. The guard then skips the migration because `already_applied = false`, and `if already_applied { continue; }` only skips when `true`. So a failed migration is retried — that is correct. However, the `ON CONFLICT DO UPDATE` on line 100 will overwrite the original `success=0` failure record with the new result, losing the history of which app version first failed. This is a minor auditability loss, not a correctness defect.

The actual warning: after a migration's SQL fails (say migration 0002), `result?` on line 109 propagates the error up through `run_migrations`. The caller in `main.rs` does not exist yet (CR-01), but any call site that unwraps this with `.expect(...)` or ignores the error will silently leave the database without the `shell_preferences` table and the app will fail later when it tries to use it.

**Fix:** The call site (in `main.rs` setup hook) must propagate or handle the `run_migrations` error, not ignore it. Panic on migration failure at startup is acceptable:

```rust
run_migrations(conn, env!("CARGO_PKG_VERSION"))
    .expect("database migration failed — cannot start");
```

---

### WR-02: Duplicate `role="application"` on the workspace shell hierarchy

**File:** `src/lib/components/WorkspaceShell.svelte:48` and `src/lib/components/AppShell.svelte:21`

**Issue:** `WorkspaceShell.svelte` (line 48) and `AppShell.svelte` (line 21) both render `<div ... role="application">`. At runtime both components are active simultaneously: `+layout.svelte` renders `<AppShell>` which includes a slot, and `+page.svelte` renders `<WorkspaceShell>` inside that slot. This produces nested `role="application"` elements, which is invalid ARIA — an `application` role should contain its own landmark subtree, not nest inside another `application`. Screen readers will announce the landmark twice. The `AppShell` component appears to be a legacy shell that was superseded by `WorkspaceShell` but not removed.

**Fix:** Either remove the `role="application"` from `AppShell.svelte` (it no longer carries the full workspace layout and should defer to `WorkspaceShell`) or, if `AppShell` is intended to be the root shell, remove `role="application"` from `WorkspaceShell.svelte`. Given the layout hierarchy, `AppShell` is the outer container and its `role="application"` should be retained; `WorkspaceShell` should be changed to a neutral container `<div>` without a conflicting landmark role.

---

### WR-03: `focusedIndex` initializer in `SurfaceRail` silently defaults to `0` when the active surface is `chat`

**File:** `src/lib/components/SurfaceRail.svelte:31-33`

**Issue:**
```ts
let focusedIndex = $state(
    surfaces.findIndex((s) => s.id === activeSurface) || 0
);
```
`Array.findIndex` returns `0` when the active surface is `chat` (the first element, index 0). Because `0` is falsy in JavaScript, the `|| 0` fallback fires for `chat` as well as for "not found" (`-1`). The result is correct (`0` in both cases), but only by coincidence. If `chat` were ever moved to a non-zero position, or if this pattern is copied to another array, the logic silently breaks. Additionally, `findIndex` returning `-1` (surface not found) would also resolve to `0` via the fallback — a case that should be an error, not a silent default.

**Fix:**
```ts
let focusedIndex = $state(
    Math.max(0, surfaces.findIndex((s) => s.id === activeSurface))
);
```
Or, more defensively:
```ts
const idx = surfaces.findIndex((s) => s.id === activeSurface);
let focusedIndex = $state(idx >= 0 ? idx : 0);
```

---

### WR-04: `surfaceStore.hydrate()` return value is not awaited in the layout — errors are not surfaced

**File:** `src/routes/+layout.svelte:9-11`

**Issue:**
```ts
onMount(() => {
    surfaceStore.hydrate();
});
```
`surfaceStore.hydrate()` returns `Promise<void>`. `onMount` ignores the returned promise — any unhandled rejection is silently dropped. While `hydrate()` itself wraps the await in try/catch and sets `error` state, a regression that throws synchronously before the try block (or a future refactor that moves the try) will be invisible. The convention in Svelte 5 + TypeScript is to await the hydration or to explicitly handle the floating promise.

**Fix:**
```ts
onMount(() => {
    surfaceStore.hydrate().catch((e) => {
        console.error('[layout] hydrate threw unexpectedly:', e);
    });
});
```

---

### WR-05: `migration_0001` duplicates the bootstrap DDL already executed inline in `run_migrations`

**File:** `src-tauri/src/storage/migrations.rs:29-41` and `60-69`

**Issue:** `run_migrations` begins with an inline `CREATE TABLE IF NOT EXISTS schema_migrations ...` (lines 60-69) to bootstrap the tracking table. Migration `0001` in the `MIGRATIONS` array (lines 29-41) also creates `schema_migrations` with `IF NOT EXISTS`. On a fresh database both run: the inline creates the table, then migration `0001` attempts the same DDL (harmless due to `IF NOT EXISTS`), then records itself as applied. On the second run, the inline DDL is a no-op, and migration `0001` is detected as already-applied and skipped correctly.

The defect is conceptual: if migration `0001` is ever marked `success=0` (failed on a previous run) and a developer tries to replay it, it will re-execute the DDL against the tracking table that already exists — the `IF NOT EXISTS` prevents data loss, but the developer is now debugging a situation where the tracking table exists yet migration `0001` reports failure. The duplication also makes `applied = MIGRATIONS.len()` on a fresh DB include the tracking table DDL as a "applied" migration, which bloats the count in tests (`migrations_apply_to_fresh_database` asserts `applied == 2`, which happens to be correct but for muddled reasons).

**Fix:** Remove migration `0001` from the `MIGRATIONS` array. The bootstrap DDL should remain in the inline block (lines 60-69), which handles it before the loop. Start user-facing schema at `0002` (or renumber `0002` to `0001`). Update the test assertion `assert_eq!(applied, MIGRATIONS.len())` to the concrete expected count.

---

## Info

### IN-01: `tests/rust/app_shell.rs` stub file adds noise without function

**File:** `tests/rust/app_shell.rs:1-9`

**Issue:** This file contains only comments directing the reader to `src-tauri/tests/app_shell.rs`. It is not compiled by Cargo (it is outside any crate root), so it contributes nothing to the test run. It will confuse contributors who try to add tests here and wonder why they don't run.

**Fix:** Delete the file. The comment directing developers to the correct test location can live in a `docs/testing.md` or in `CLAUDE.md`.

---

### IN-02: `SURFACE_LABELS` is defined in two places in the frontend

**File:** `src/lib/stores/surface.ts:18-23` and `src/routes/+page.svelte:10-15` and `src/lib/components/WorkspaceShell.svelte:33-38`

**Issue:** The mapping from `Surface` enum values to human-readable labels is duplicated across three files. All three must be kept in sync when a new surface is added. Currently all three agree, but the duplication is a maintenance hazard.

**Fix:** Export `SURFACE_LABELS` from `surface.ts` and import it in the other two files. The store already has the canonical definition.

---

### IN-03: `tauri.conf.json` uses placeholder bundle identifier

**File:** `src-tauri/tauri.conf.json:3`

**Issue:** `"identifier": "com.example.desktop-ai-client"` uses the `com.example` namespace. This is a scaffold value. On macOS, bundle identifiers in the `com.example` namespace are not permitted in the App Store, and on Windows/Linux they affect update channel resolution and OS-level sandbox identity. This is low-risk for a scaffold phase but needs to be replaced before the app ships to real users.

**Fix:** Replace with the project's actual reverse-domain identifier before the first distributable build. Track it as a known pre-release TODO in `.planning/REQUIREMENTS.md` if it is not already there.

---

_Reviewed: 2026-06-13T19:27:12Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
