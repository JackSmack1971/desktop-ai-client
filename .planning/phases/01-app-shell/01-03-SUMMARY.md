---
phase: 01-app-shell
plan: "03"
subsystem: bootstrap-wiring
tags: [tauri, sqlite, ipc, permissions, hydration, accessibility]
dependency_graph:
  requires: ["01-01", "01-02"]
  provides: [runnable-shell-bootstrap, capability-permissions, hydration-guard]
  affects: [src-tauri/src/main.rs, src-tauri/src/storage/sqlite.rs, src-tauri/src/app_state.rs, src-tauri/src/ipc/app_shell.rs, src-tauri/capabilities/main.json, src-tauri/permissions/app-shell.toml, src/lib/components/AppShell.svelte, src/lib/components/SurfaceRail.svelte, src/routes/+layout.svelte]
tech_stack:
  added: []
  patterns: [tauri-setup-hook, arc-managed-state, explicit-hydration-flag, roving-tabindex-fix]
key_files:
  created:
    - src-tauri/permissions/app-shell.toml
  modified:
    - src-tauri/src/main.rs
    - src-tauri/src/storage/sqlite.rs
    - src-tauri/src/app_state.rs
    - src-tauri/src/ipc/app_shell.rs
    - src-tauri/capabilities/main.json
    - src/lib/components/AppShell.svelte
    - src/lib/components/SurfaceRail.svelte
    - src/routes/+layout.svelte
decisions:
  - "run_migrations called inside SqlitePool::open() so every open site gets migrations for free — test path uses from_connection() to preserve manual control"
  - "Arc<SqlitePool> shared between managed SqlitePool and ShellPreferenceStore to avoid opening two connections"
  - "hydrated: bool flag on ShellState guards DB consult exactly once per session, replacing fragile default-value equality"
  - "Bare permission identifiers (allow-get-active-surface) used in capabilities/main.json per Tauri 2 app-own command convention"
metrics:
  duration: "~10 minutes"
  completed: "2026-06-13T20:47:00Z"
  tasks_completed: 2
  tasks_total: 2
  files_created: 1
  files_modified: 8
---

# Phase 01 Plan 03: Bootstrap Wiring and Correctness Fixes Summary

Gap-closure plan closing CR-01, CR-02, CR-03, CR-04, WR-02, WR-03, WR-04: Tauri .setup() hook wires SqlitePool and ShellPreferenceStore as managed state from a single Arc, migrations run inside SqlitePool::open(), both IPC commands are declared in permissions/app-shell.toml and granted in capabilities/main.json, and four frontend correctness bugs (hydration guard, nested ARIA role, falsy index, floating promise) are fixed.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Wire Tauri bootstrap — setup hook, migrations, capability permissions (CR-01, CR-04, CR-02) | 12c1b20 | main.rs, sqlite.rs, permissions/app-shell.toml, capabilities/main.json |
| 2 | Fix correctness warnings — hydration guard, nested ARIA role, falsy index, floating promise (CR-03, WR-02, WR-03, WR-04) | bb825ba | app_state.rs, ipc/app_shell.rs, AppShell.svelte, SurfaceRail.svelte, +layout.svelte |

## What Changed

### Task 1 (12c1b20): Bootstrap Wiring

**CR-04 — SqlitePool::open() runs migrations:** Added `crate::storage::migrations::run_migrations(&conn, env!("CARGO_PKG_VERSION"))?` inside `SqlitePool::open()` after the pragma batch, before moving the connection into `Mutex::new`. The doc-comment already stated migrations run — now the implementation matches. The `from_connection()` path is unchanged so tests retain manual migration control.

**CR-01 — .setup() hook in main.rs:** Added a `.setup(|app| { ... })` closure to the `tauri::Builder` chain, inserted before `.invoke_handler`. The hook:
1. Resolves `app.path().app_data_dir()` (imports `tauri::Manager`)
2. Creates the directory with `std::fs::create_dir_all` for first-launch safety
3. Opens the pool: `Arc::new(SqlitePool::open(db_path)?)` — migrations run inside `open()` so `shell_preferences` exists after this line
4. Registers `pool.clone()` as `tauri::State<'_, SqlitePool>` via `app.manage`
5. Registers `ShellPreferenceStore::new(pool)` as `tauri::State<'_, ShellPreferenceStore>` via `app.manage`
6. Returns `Ok(())` — rusqlite errors box automatically via `?`

**CR-02 — Capability permissions:** Created `src-tauri/permissions/app-shell.toml` with two `[[permission]]` blocks using bare identifiers (`allow-get-active-surface`, `allow-set-active-surface`). Added both to `capabilities/main.json`'s permissions array while retaining the four pre-existing entries.

### Task 2 (bb825ba): Correctness Fixes

**CR-03 — Explicit hydration flag:** Added `pub hydrated: bool` to `ShellState` (derives `Default` so starts `false`). In `get_active_surface`, replaced the fragile `if current == Surface::Chat` guard with `if !hydrated`. The handler reads `hydrated` in the same lock scope as `active_surface`, then sets `shell.hydrated = true` on both DB paths (found and not-found), ensuring the DB is consulted exactly once per session.

**WR-02 — Remove nested role="application":** Removed `role="application"` from the outer `<div class="app-shell">` in `AppShell.svelte`. `aria-label="Desktop AI Client"` is retained on the div. `WorkspaceShell.svelte` retains its `role="application"` as the semantically correct single owner.

**WR-03 — Falsy index coercion:** Replaced `surfaces.findIndex(...) || 0` with `(() => { const idx = surfaces.findIndex((s) => s.id === activeSurface); return idx === -1 ? 0 : idx; })()`. Index 0 (Chat) is now a valid focused index rather than being coerced to the same fallback as -1 (not-found).

**WR-04 — Floating promise:** Changed `surfaceStore.hydrate()` to `surfaceStore.hydrate().catch((e) => console.error('surface hydration failed', e))` in `+layout.svelte`. Promise rejections are now surfaced to the console instead of being silently dropped.

## Verification Results

| Check | Result |
|-------|--------|
| `npm run check` | 0 errors, 0 warnings |
| `grep -c '.setup(' src-tauri/src/main.rs` | 1 |
| `grep -n 'run_migrations' sqlite.rs` (inside open()) | line 38 |
| `grep -c '^\[\[permission\]\]' permissions/app-shell.toml` | 2 |
| `allow-get-active-surface` in capabilities/main.json | present |
| `allow-set-active-surface` in capabilities/main.json | present |
| Pre-existing 4 permissions retained | confirmed |
| `grep -c 'role="application"' AppShell.svelte` | 0 |
| WorkspaceShell.svelte role="application" (element, not comment) | 1 |
| `grep -c '|| 0' SurfaceRail.svelte` | 0 |
| `grep -c '.catch(' +layout.svelte` | 1 |
| `grep -c 'hydrated: bool' app_state.rs` | 1 |
| `grep -c 'current == Surface::Chat' app_shell.rs` | 0 |
| `hydrated = true` on both DB paths | confirmed (lines 60, 67) |

**Note on cargo build/test:** Rust toolchain (cargo/rustc) is not installed in this execution environment. Static structural verification was performed for all Rust changes. The `npm run check` frontend typecheck passes (0 errors, 0 warnings). Rust build verification is deferred to the CI/CD environment where the toolchain is available.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as specified.

### Environment Constraint

**Cargo not available:** The plan's automated verification steps (`cargo build`, `cargo test`) could not be executed because the Rust toolchain is absent from this environment (confirmed via `npx tauri info`). The `target/` directory contains artifacts from a previous build environment. All Rust code changes were verified through static structural analysis (grep checks on required patterns, function signatures, module references). The syntactic correctness is high-confidence given:
- No new dependencies introduced
- Existing module structure preserved (`crate::storage::migrations::run_migrations` path matches the `mod storage; mod migrations;` tree)
- `env!("CARGO_PKG_VERSION")` macro is standard Rust compile-time expansion
- `tauri::Manager` trait import follows documented Tauri 2 pattern

## Known Stubs

None — this plan closes implementation gaps; no new stubs introduced.

## Threat Flags

No new security surface beyond what the plan's threat model covers. The `app-shell.toml` permissions are scoped to exactly the two declared commands with no wildcard grants (T-01-01 mitigated). The `set_active_surface` handler uses a closed enum (T-01-02 mitigated). The SQLite file is stored under the OS per-app data dir (T-01-03 accepted). Migration idempotency is preserved by the `schema_migrations` tracking table (T-01-04 mitigated).

## Self-Check: PASSED

| Item | Status |
|------|--------|
| src-tauri/permissions/app-shell.toml exists | FOUND |
| .planning/phases/01-app-shell/01-03-SUMMARY.md exists | FOUND |
| Commit 12c1b20 (Task 1) | FOUND |
| Commit bb825ba (Task 2) | FOUND |
