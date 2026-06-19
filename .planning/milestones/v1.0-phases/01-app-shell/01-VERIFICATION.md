---
phase: 01-app-shell
verified: 2026-06-13T21:30:00Z
status: human_needed
score: 7/7 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/7
  gaps_closed:
    - 'The app opens into a usable desktop shell instead of a blank scaffold'
    - 'The shell loads a backend-owned active-surface preference on startup'
    - 'The active surface survives a restart through backend-owned persistence'
  gaps_remaining: []
  regressions: []
human_verification:
  - test: 'Run npm install && npm run dev and observe the launched app'
    expected: 'App opens into the workspace shell showing the SurfaceRail with four labeled tabs (Chat, History, Settings, Artifacts) and Chat surface as default. No runtime panics, no IPC error messages in the StatusRegion on first launch.'
    why_human: 'Requires Tauri runtime (native window), visual inspection, and DevTools console check. Cargo/Rust build verification also deferred to CI.'
  - test: 'Keyboard-only surface navigation'
    expected: 'Tab moves to skip-to-content link, then to the rail, then into the panel. Arrow Up/Down move focus between rail tabs. Enter/Space activate the focused tab and move focus to #main-content. Home jumps to Chat tab, End jumps to Artifacts tab. Focus ring (2px blue outline) visible on every focused element.'
    why_human: 'ARIA APG keyboard interaction can only be confirmed through actual keyboard use in a running browser/WebView.'
  - test: 'Screen reader surface announcements on switch'
    expected: "Screen reader announces the surface name when focus moves to a tab (via aria-label), announces 'selected' state change (via aria-selected), and announces non-interruptively via StatusRegion's aria-live=polite region. No duplicate 'application' landmark announcements — only WorkspaceShell has role=application."
    why_human: 'AT behavior depends on specific screen reader implementations and browser/WebView combination.'
  - test: 'Surface persistence across restarts'
    expected: 'Switch to History surface, quit the app, relaunch — shell opens to History surface confirming SQLite persistence round-trip through the full IPC stack.'
    why_human: 'Requires actual process lifecycle (quit + relaunch) and visual inspection of restored state.'
  - test: 'cargo test --workspace --all-targets'
    expected: 'All 5 integration tests in src-tauri/tests/app_shell.rs pass plus all inline unit tests in app_state.rs, storage/sqlite.rs, storage/migrations.rs, and ipc/app_shell.rs. No test failures or panics.'
    why_human: 'Cargo/Rust not installed in verification environment. Tests are structurally correct and cover the required behaviors but have not been executed.'
---

# Phase 01: App Shell Verification Report

**Phase Goal:** Get the desktop app booting into a usable workspace shell with clear navigation boundaries.
**Verified:** 2026-06-13T21:30:00Z
**Status:** human_needed
**Re-verification:** Yes — after gap closure plan 01-03

All three runtime blockers (CR-01, CR-02, CR-04) and four correctness warnings (CR-03, WR-02, WR-03, WR-04) documented in the previous verification have been closed by plan 01-03. All 7 must-have truths are now verified at the code level. Automated checks pass where runnable (npm run check); cargo build and runtime tests remain deferred to human/CI.

---

## Goal Achievement

### Observable Truths

| #   | Truth                                                                                      | Status   | Evidence                                                                                                                                                                                                                                                                                                                                           |
| --- | ------------------------------------------------------------------------------------------ | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | The app opens into a usable desktop shell instead of a blank scaffold                      | VERIFIED | `main.rs` has `.setup()` hook (line 24) resolving `app_data_dir`, calling `SqlitePool::open()`, and registering both `SqlitePool` and `ShellPreferenceStore` via `app.manage()`. Migrations run inside `open()` at `sqlite.rs:38`. Capability grant in `capabilities/main.json` includes both shell commands.                                      |
| 2   | The shell loads a backend-owned active-surface preference on startup                       | VERIFIED | `surface.ts hydrate()` calls `invoke('get_active_surface')` (IPC path now open). `get_active_surface` injects `ShellPreferenceStore` (now managed) and reads from `shell_preferences` table (now created by `run_migrations` inside `open()`). Hydration guard uses explicit `hydrated: bool` flag, not default-value equality.                    |
| 3   | Frontend navigation changes the visible surface without exposing raw file paths or secrets | VERIFIED | `surface.ts` uses `invoke()` for both reads and writes; no `localStorage`/`sessionStorage` references; `Surface` type is a closed enum in both Rust and TypeScript. `IPC` commands enforce window label as backend-side defense. No regression from previous verification.                                                                         |
| 4   | The user can switch between chat, history, settings, and artifact surfaces from the shell  | VERIFIED | `+page.svelte` routes all four surface components via `$derived(surfaceStore.surface)`. `WorkspaceShell` + `SurfaceRail` (tablist/tab) + `SurfacePanel` (tabpanel) are composed correctly. All four named surfaces are reachable.                                                                                                                  |
| 5   | Focus order and keyboard activation work for the surface switcher                          | VERIFIED | `SurfaceRail.svelte` implements roving tabindex (only active button at `tabindex=0`); `handleKeydown` handles ArrowUp/Down/Left/Right, Home, End, Enter, Space; `activate()` calls `document.getElementById('main-content')?.focus()`. `focus-visible` CSS present.                                                                                |
| 6   | The active surface survives a restart through backend-owned persistence                    | VERIFIED | `set_active_surface` IPC path is now open (CR-01 + CR-02 closed). `ShellPreferenceStore.save_active_surface` uses UPSERT. `shell_preferences` table exists after migrations. `get_active_surface` hydration guard correctly reads DB on first call (`!hydrated`) and sets `shell.hydrated = true` on both found and not-found paths.               |
| 7   | Backend smoke tests cover the write/read/restore path for the active surface               | VERIFIED | `src-tauri/tests/app_shell.rs` has 5 named integration tests: round-trip, fresh-db hydration, UPSERT overwrite, session restore, all-surfaces persistence. Tests use `SqlitePool::from_connection()` with manually-run migrations, correctly bypassing `open()` for isolated test control. Structure is substantive and covers required behaviors. |

**Score: 7/7 truths verified** (all at code level; cargo execution and runtime behavior require human/CI verification)

---

## Gap Closure Verification (Re-verification Focus)

### CR-01 — .setup() hook in main.rs

| Acceptance Criterion                                               | Result                |
| ------------------------------------------------------------------ | --------------------- |
| `.setup(` closure present                                          | CLOSED — `main.rs:24` |
| `app.path().app_data_dir()` called                                 | CLOSED — `main.rs:27` |
| `std::fs::create_dir_all` for first-launch                         | CLOSED — `main.rs:28` |
| `SqlitePool::open(db_path)?` inside setup                          | CLOSED — `main.rs:34` |
| `app.manage(pool.clone())` registers `SqlitePool`                  | CLOSED — `main.rs:37` |
| `app.manage(ShellPreferenceStore::new(pool))` registers store      | CLOSED — `main.rs:40` |
| `use tauri::Manager` imported                                      | CLOSED — `main.rs:14` |
| `use storage::sqlite::{ShellPreferenceStore, SqlitePool}` imported | CLOSED — `main.rs:13` |

### CR-02 — Capability permissions

| Acceptance Criterion                                                               | Result                                                                                                               |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/permissions/app-shell.toml` exists                                      | CLOSED — file present with 2 `[[permission]]` blocks                                                                 |
| `allow-get-active-surface` declared with `commands.allow = ["get_active_surface"]` | CLOSED                                                                                                               |
| `allow-set-active-surface` declared with `commands.allow = ["set_active_surface"]` | CLOSED                                                                                                               |
| Both permissions referenced in `capabilities/main.json`                            | CLOSED — lines 11–12                                                                                                 |
| Four pre-existing permissions retained                                             | CLOSED — `core:default`, `opener:default`, `core:app:allow-app-hide`, `core:window:allow-start-dragging` all present |

### CR-03 — Explicit hydration flag

| Acceptance Criterion                                          | Result                                                  |
| ------------------------------------------------------------- | ------------------------------------------------------- |
| `pub hydrated: bool` on `ShellState`                          | CLOSED — `app_state.rs:28`                              |
| Guard branches on `if !hydrated` (not default-value equality) | CLOSED — `app_shell.rs:54`                              |
| `shell.hydrated = true` on DB-found path                      | CLOSED — `app_shell.rs:60`                              |
| `shell.hydrated = true` on DB-not-found path                  | CLOSED — `app_shell.rs:67`                              |
| `current == Surface::Chat` guard removed                      | CLOSED — `grep -c 'current == Surface::Chat'` returns 0 |

### CR-04 — Migrations inside SqlitePool::open()

| Acceptance Criterion                                     | Result                                                                                                        |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `run_migrations` called inside `open()` body             | CLOSED — `sqlite.rs:38`                                                                                       |
| Call is before `Mutex::new(conn)`                        | CLOSED — call is on the local `conn` binding at line 38, `Ok(Self { conn: Mutex::new(conn) })` at lines 40-42 |
| Doc-comment on `open()` accurately states migrations run | CLOSED — lines 22-24 match implementation                                                                     |
| `from_connection()` path unchanged                       | VERIFIED — no migration call in `from_connection()`, preserving test control                                  |

### WR-02 — Nested role="application" removed

| Acceptance Criterion                                     | Result                                                                              |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `role="application"` absent from `AppShell.svelte`       | CLOSED — line 21 has no `role` attribute; `aria-label="Desktop AI Client"` retained |
| `role="application"` retained in `WorkspaceShell.svelte` | VERIFIED — `WorkspaceShell.svelte:48` retains `role="application"` as sole owner    |

### WR-03 — Falsy index coercion fixed

| Acceptance Criterion                     | Result                                                                   |
| ---------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------ | ------------- | --- | ---------- |
| `                                        |                                                                          | 0`pattern removed from`SurfaceRail.svelte` | CLOSED — no ` |     | 0` present |
| Explicit `=== -1 ? 0 : idx` ternary used | CLOSED — `SurfaceRail.svelte:32` uses IIFE with explicit not-found check |

### WR-04 — Floating promise handled

| Acceptance Criterion                                                | Result                                                                                         |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `.catch()` attached to `surfaceStore.hydrate()` in `+layout.svelte` | CLOSED — `+layout.svelte:10` has `.catch((e) => console.error('surface hydration failed', e))` |
| Bare un-chained `surfaceStore.hydrate()` gone                       | CLOSED — no bare call without `.catch`                                                         |

---

## Required Artifacts

| Artifact                                   | Expected                                                        | Status   | Details                                                                                 |
| ------------------------------------------ | --------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------- | --- | ---------- |
| `package.json`                             | Build manifests with dev/build/check scripts                    | VERIFIED | dev, build, check, frontend:dev, frontend:build scripts present (no regression)         |
| `src-tauri/Cargo.toml`                     | Rust manifest with rusqlite, tokio, serde, tauri 2              | VERIFIED | All dependencies present including rusqlite bundled feature (no regression)             |
| `src-tauri/src/main.rs`                    | Thin bootstrap with .setup() hook registering all managed state | VERIFIED | `.setup()` closes all three runtime blockers; 57-line file with clear module boundaries |
| `src-tauri/permissions/app-shell.toml`     | Two [[permission]] blocks for shell IPC commands                | VERIFIED | New file; both identifiers declared with correct `commands.allow` arrays                |
| `src-tauri/capabilities/main.json`         | Capability grant including shell IPC commands                   | VERIFIED | Both shell commands added; four pre-existing permissions retained                       |
| `src-tauri/src/app_state.rs`               | ShellState with hydrated: bool field                            | VERIFIED | `pub hydrated: bool` with doc-comment on line 28                                        |
| `src-tauri/src/ipc/app_shell.rs`           | Typed IPC with hydration guard on explicit flag                 | VERIFIED | `if !hydrated` guard; `hydrated = true` on both DB paths; no default-value equality     |
| `src-tauri/src/storage/sqlite.rs`          | SqlitePool::open() runs migrations                              | VERIFIED | `run_migrations` called at line 38 inside `open()`; doc-comment matches                 |
| `src/routes/+layout.svelte`                | Layout with caught hydrate() call                               | VERIFIED | `.catch()` attached; promise is no longer floating                                      |
| `src/lib/components/AppShell.svelte`       | Root layout without nested role="application"                   | VERIFIED | `role="application"` removed; `aria-label` retained                                     |
| `src/lib/components/SurfaceRail.svelte`    | Roving tabindex with explicit index check                       | VERIFIED | IIFE explicit `-1` check; `                                                             |     | 0` removed |
| `src/lib/components/WorkspaceShell.svelte` | Composed layout with sole role="application"                    | VERIFIED | Single `role="application"` at line 48                                                  |
| `src/lib/stores/surface.ts`                | Svelte 5 store with IPC hydration                               | VERIFIED | No regression; `invoke()` for both commands, optimistic rollback, no browser storage    |
| `src-tauri/tests/app_shell.rs`             | 5 integration tests covering persistence round trip             | VERIFIED | All 5 tests structurally complete and substantive; not executed (no Cargo)              |

---

## Key Link Verification

| From                                       | To                                                    | Via                                                                 | Status | Details                                                                           |
| ------------------------------------------ | ----------------------------------------------------- | ------------------------------------------------------------------- | ------ | --------------------------------------------------------------------------------- |
| `surface.ts hydrate()`                     | `get_active_surface` Rust command                     | `invoke('get_active_surface')`                                      | WIRED  | Frontend call correct; capability grant added; `ShellPreferenceStore` now managed |
| `surface.ts setSurface()`                  | `set_active_surface` Rust command                     | `invoke('set_active_surface', { surface })`                         | WIRED  | Same as above; optimistic rollback on failure                                     |
| `main.rs .setup()`                         | `SqlitePool::open()` + `run_migrations`               | `let pool = Arc::new(SqlitePool::open(db_path)?)`                   | WIRED  | Migrations run inside `open()` before return                                      |
| `main.rs .setup()`                         | `app.manage(pool.clone())`                            | `tauri::State<'_, SqlitePool>`                                      | WIRED  | Registered at `main.rs:37`                                                        |
| `main.rs .setup()`                         | `app.manage(ShellPreferenceStore::new(pool))`         | `tauri::State<'_, ShellPreferenceStore>`                            | WIRED  | Registered at `main.rs:40`                                                        |
| `capabilities/main.json`                   | `permissions/app-shell.toml`                          | `allow-get-active-surface` + `allow-set-active-surface` identifiers | WIRED  | Both bare identifiers declared in TOML; referenced in JSON                        |
| `ipc/app_shell.rs get_active_surface`      | `ShellState.hydrated`                                 | `if !hydrated` guard                                                | WIRED  | Explicit flag replaces default-value equality; set on both DB paths               |
| `ShellPreferenceStore.save_active_surface` | `shell_preferences` SQL table                         | `INSERT INTO shell_preferences ... ON CONFLICT DO UPDATE`           | WIRED  | Table exists (migrations run); UPSERT correct                                     |
| `ShellPreferenceStore.load_active_surface` | `shell_preferences` SQL table                         | `SELECT active_surface FROM shell_preferences WHERE id = 1`         | WIRED  | Table exists; `QueryReturnedNoRows` returns `Ok(None)` correctly                  |
| `SurfaceRail tab buttons`                  | `SurfacePanel#surface-panel`                          | `aria-controls="surface-panel"`                                     | WIRED  | No regression                                                                     |
| `+layout.svelte onMount`                   | `surfaceStore.hydrate()`                              | `.catch((e) => console.error(...))`                                 | WIRED  | Promise no longer floating                                                        |
| `+page.svelte`                             | `WorkspaceShell > SurfacePanel > {surface component}` | `$derived(surfaceStore.surface)`                                    | WIRED  | All four surfaces routed conditionally; no regression                             |

---

## Data-Flow Trace (Level 4)

| Artifact              | Data Variable      | Source                                          | Produces Real Data                                                            | Status                                      |
| --------------------- | ------------------ | ----------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------- |
| `surface.ts`          | `surface` ($state) | `invoke('get_active_surface')` in `hydrate()`   | Yes — IPC path now fully open (managed state + capability grant + migrations) | FLOWING (code-verified; runtime unverified) |
| `StatusRegion.svelte` | `announceText`     | `message`, `loading`, `error` from surfaceStore | Reactive from store; store data flows from real IPC call                      | FLOWING                                     |
| `SurfaceRail.svelte`  | `activeSurface`    | `$derived(surfaceStore.surface)`                | Reactive from store                                                           | FLOWING                                     |

---

## Behavioral Spot-Checks

Step 7b: Partial — Tauri desktop runtime not available.

| Behavior                          | Command                                                             | Result                                                  | Status                                                 |
| --------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------ |
| Frontend typecheck                | `npm run check`                                                     | 0 errors, 0 warnings (executor-reported for plan 01-03) | SKIP — not re-runnable in this environment             |
| Rust unit/integration tests       | `cargo test --workspace --all-targets`                              | NOT RUN                                                 | SKIP — Cargo not installed in verification environment |
| App launches and hydrates         | `npm run dev`                                                       | NOT RUN                                                 | SKIP — requires Tauri desktop runtime                  |
| setup() hook pattern present      | `grep -c '.setup(' src-tauri/src/main.rs`                           | 1 (verified by file read)                               | PASS                                                   |
| run_migrations in open()          | `grep -n 'run_migrations' src-tauri/src/storage/sqlite.rs`          | line 38 (verified by file read)                         | PASS                                                   |
| Both IPC commands in capabilities | `capabilities/main.json` permissions array                          | both present (verified by file read)                    | PASS                                                   |
| Permission TOML has 2 blocks      | `grep -c '^\[\[permission\]\]' permissions/app-shell.toml`          | 2 (verified by file read)                               | PASS                                                   |
| Hydrated flag present             | `grep -c 'hydrated: bool' src-tauri/src/app_state.rs`               | 1 (verified by file read)                               | PASS                                                   |
| Old Chat equality guard gone      | `grep -c 'current == Surface::Chat' src-tauri/src/ipc/app_shell.rs` | 0 (verified by file read)                               | PASS                                                   |
| No nested role="application"      | `grep -c 'role="application"' src/lib/components/AppShell.svelte`   | 0 (verified by file read)                               | PASS                                                   |
| Falsy coercion removed            | `grep -c '\|\| 0' src/lib/components/SurfaceRail.svelte`            | 0 (verified by file read)                               | PASS                                                   |
| Promise catch present             | `grep -n '.catch(' src/routes/+layout.svelte`                       | line 10 (verified by file read)                         | PASS                                                   |

---

## Probe Execution

No `scripts/*/tests/probe-*.sh` files declared in any PLAN or found in the repository.

---

## Requirements Coverage

| Requirement | Source Plan                  | Description                                                              | Status           | Evidence                                                                                                                                                                                                                                                                                                 |
| ----------- | ---------------------------- | ------------------------------------------------------------------------ | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SHELL-01    | 01-01-PLAN.md, 01-03-PLAN.md | User can launch the desktop app and reach the main workspace             | SATISFIED (code) | Bootstrap wiring complete: `.setup()` hook opens SQLite, runs migrations, manages both storage types. Capability grant unblocks IPC. App can structurally launch into workspace shell. Runtime confirmation deferred to human verification.                                                              |
| SHELL-02    | 01-02-PLAN.md, 01-03-PLAN.md | User can navigate between chat, history, settings, and artifact surfaces | SATISFIED (code) | `WorkspaceShell` + `SurfaceRail` (ARIA tablist/tab) + `SurfacePanel` (tabpanel) implements accessible four-surface navigation. Keyboard (roving tabindex, Arrow/Home/End/Enter/Space) fully wired. Surface preference persists through backend IPC. Runtime confirmation deferred to human verification. |

### Roadmap Success Criteria Coverage

| SC  | Text                                                                        | Status           | Evidence                                                                                                                      |
| --- | --------------------------------------------------------------------------- | ---------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| 1   | The app launches successfully from the desktop shell                        | SATISFIED (code) | Bootstrap complete; no structural blockers remain                                                                             |
| 2   | The user can reach the main workspace and switch between the major surfaces | SATISFIED (code) | All four surfaces reachable via `+page.svelte` routing and `SurfaceRail`                                                      |
| 3   | The shell is organized so backend and frontend boundaries remain explicit   | SATISFIED        | IPC commands are the sole cross-boundary path; no browser storage; Surface is a closed enum; window label enforced by backend |

---

## Anti-Patterns Found

| File                                                  | Line | Pattern             | Severity | Impact                                                                                             |
| ----------------------------------------------------- | ---- | ------------------- | -------- | -------------------------------------------------------------------------------------------------- |
| `src/lib/components/surfaces/ChatSurface.svelte`      | —    | Placeholder content | INFO     | Intentional scaffold — full conversation UI is out-of-scope for Phase 1 per plan 01-01 known stubs |
| `src/lib/components/surfaces/HistorySurface.svelte`   | —    | Placeholder content | INFO     | Same — intentional scaffold                                                                        |
| `src/lib/components/surfaces/SettingsSurface.svelte`  | —    | Placeholder content | INFO     | Same — intentional scaffold                                                                        |
| `src/lib/components/surfaces/ArtifactsSurface.svelte` | —    | Placeholder content | INFO     | Same — intentional scaffold                                                                        |

No `TBD`, `FIXME`, or `XXX` markers found in any file modified by plans 01-01, 01-02, or 01-03. No blockers.

The four surface scaffold components render placeholder text. This is intentional and documented in plan summaries — they prove shell routing works. Full surface implementations are scoped to later phases (Phase 2+).

---

## Human Verification Required

The following items require a runtime environment or assistive technology that cannot be verified from static code analysis.

### 1. App Launch and Surface Display

**Test:** Run `npm install && npm run dev` (requires Node.js, npm, Rust/Cargo, and Tauri build tools installed) and observe the launched desktop app.
**Expected:** App opens into the workspace shell showing the SurfaceRail with four labeled tabs (Chat, History, Settings, Artifacts) and Chat as the default surface. No runtime panics, no IPC error messages in the StatusRegion or browser DevTools console. IPC calls to `get_active_surface` succeed rather than being blocked or panicking.
**Why human:** Requires Tauri native desktop runtime, visual inspection, and DevTools console monitoring.

### 2. Keyboard-Only Surface Navigation

**Test:** With the app running, navigate using only Tab, Arrow keys, Enter, Space, Home, End.
**Expected:** Tab moves to the skip-to-content link, then to the rail, then into the panel. Arrow Up/Down move focus between rail tabs. Enter/Space activate the focused tab and move focus to `#main-content`. Home jumps to Chat tab, End jumps to Artifacts tab. Focus ring (2px blue outline) visible on every focused element.
**Why human:** ARIA APG keyboard interaction can only be confirmed through actual keyboard use in a running browser/WebView.

### 3. Screen Reader Surface Announcements

**Test:** With a screen reader (VoiceOver/macOS, NVDA/Windows, or Orca/Linux) active, switch surfaces via keyboard.
**Expected:** Screen reader announces the surface name on tab focus (via `aria-label`), announces "selected" state on activation (via `aria-selected`), and announces surface change non-interruptively via the StatusRegion's `aria-live="polite"` region. Only one `role="application"` appears in the AT tree — `WorkspaceShell` only.
**Why human:** AT behavior depends on specific screen reader and browser/WebView combination.

### 4. Surface Persistence Across Restarts

**Test:** After the app launches successfully (Human Check 1), switch to the History surface, quit the app, relaunch.
**Expected:** The app opens to the History surface (not Chat), confirming the SQLite persistence round-trip works through the full IPC stack.
**Why human:** Requires actual process lifecycle (quit + relaunch) and visual inspection of restored state.

### 5. cargo test Execution

**Test:** In an environment with Rust/Cargo installed, run `cargo test --workspace --all-targets` from the repository root.
**Expected:** All 5 integration tests in `src-tauri/tests/app_shell.rs` pass plus all inline unit tests in `app_state.rs`, `storage/sqlite.rs`, `storage/migrations.rs`, and `ipc/app_shell.rs`. No test failures or panics.
**Why human:** Cargo not installed in verification environment. Tests are structurally correct and cover all required behaviors, but have not been executed by any executor or this verifier.

---

## Re-verification Summary

**Previous status:** gaps_found (3/7)
**Current status:** human_needed (7/7)

All three runtime blockers that prevented Phase 1 from achieving its goal have been closed by plan 01-03:

- **CR-01 (setup hook):** `main.rs` now has a complete `.setup()` closure that resolves `app_data_dir`, opens `SqlitePool`, runs migrations, and registers both `SqlitePool` and `ShellPreferenceStore` as managed Tauri state.
- **CR-02 (capability permissions):** `permissions/app-shell.toml` declares both IPC commands; `capabilities/main.json` grants them while retaining all four pre-existing permissions.
- **CR-04 (migrations in open()):** `SqlitePool::open()` calls `run_migrations` before wrapping the connection; doc-comment now matches implementation.

All four correctness warnings have also been resolved:

- **CR-03:** `ShellState.hydrated` flag guards DB consult exactly once; old default-value equality removed.
- **WR-02:** `role="application"` removed from `AppShell.svelte`; sole owner is `WorkspaceShell.svelte`.
- **WR-03:** Explicit `-1` check replaces falsy `|| 0` coercion in `SurfaceRail.svelte`.
- **WR-04:** `.catch()` attached to `hydrate()` call in `+layout.svelte`.

No regressions found in the four truths that previously passed (truths 3, 4, 5, 7).

The remaining gap is environmental: the Rust toolchain is not available in this verification environment, so `cargo build`, `cargo test`, and `npm run dev` cannot be executed. All human verification items are contingent on having the full Tauri build environment available.

---

_Verified: 2026-06-13T21:30:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification of: 01-VERIFICATION.md (previous status: gaps_found, 2026-06-13T20:00:00Z)_
