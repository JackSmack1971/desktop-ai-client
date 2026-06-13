---
phase: 01-app-shell
verified: 2026-06-13T20:00:00Z
status: gaps_found
score: 3/7 must-haves verified
overrides_applied: 0
gaps:
  - truth: "The app opens into a usable desktop shell instead of a blank scaffold."
    status: failed
    reason: |
      Three compounding blockers prevent the app from launching into a working shell:
      (1) ShellPreferenceStore and SqlitePool are never registered with Tauri .manage() —
      both IPC commands will panic at runtime with "unmanaged state".
      (2) get_active_surface and set_active_surface are absent from capabilities/main.json —
      Tauri 2's deny-by-default permission system blocks both commands before the Rust
      handler is ever reached.
      (3) SqlitePool::open() does not call run_migrations despite the doc-comment claiming
      it does — on first launch shell_preferences table does not exist.
    artifacts:
      - path: "src-tauri/src/main.rs"
        issue: "Only manages AppState::default(). No .setup() hook, no SqlitePool::open(), no ShellPreferenceStore::new(), no app.manage() for either storage type."
      - path: "src-tauri/capabilities/main.json"
        issue: "Permissions array contains only core:default, opener:default, core:app:allow-app-hide, core:window:allow-start-dragging. get_active_surface and set_active_surface are absent."
      - path: "src-tauri/src/storage/sqlite.rs"
        issue: "SqlitePool::open() (lines 25-38) sets pragmas and returns but never calls run_migrations. Doc-comment on line 22-24 states 'Applies all pending migrations before returning' — false."
    missing:
      - "Add .setup() hook in main.rs that resolves app_data_dir, calls SqlitePool::open(), calls run_migrations, wraps pool in Arc, and registers both SqlitePool and ShellPreferenceStore via app.manage()"
      - "Add get_active_surface and set_active_surface to capabilities/main.json permissions array (or declare a custom permission set in src-tauri/permissions/ and reference it)"
      - "Either call run_migrations inside SqlitePool::open() or ensure every call site calls it immediately after open(). The doc-comment contract must match the implementation."

  - truth: "The shell loads a backend-owned active-surface preference on startup."
    status: failed
    reason: |
      Startup hydration requires three things to work in sequence: (1) SqlitePool opened
      with migrations run so shell_preferences table exists, (2) ShellPreferenceStore managed
      so Tauri can inject it into get_active_surface, (3) capability grant so the frontend
      invoke('get_active_surface') is not rejected. All three are broken (CR-01, CR-02, CR-04).
      The surface.ts store has correct hydration code — the broken layer is entirely in Rust/Tauri.
    artifacts:
      - path: "src-tauri/src/main.rs"
        issue: "ShellPreferenceStore not managed — get_active_surface panics on state injection"
      - path: "src-tauri/capabilities/main.json"
        issue: "get_active_surface not in permissions — call rejected by Tauri 2 permission layer"
      - path: "src-tauri/src/storage/sqlite.rs"
        issue: "Migrations not run by open() — shell_preferences table absent on first launch"
    missing:
      - "Fix main.rs setup hook (see gap 1 above) to register ShellPreferenceStore"
      - "Add capability permission grant (see gap 1 above)"
      - "Fix SqlitePool::open() to run migrations (see gap 1 above)"

  - truth: "The active surface survives a restart through backend-owned persistence."
    status: failed
    reason: |
      Persistence requires the IPC write path (set_active_surface) to succeed. CR-01 and CR-02
      prevent set_active_surface from ever being reached. Even if wiring were fixed, CR-04 means
      the shell_preferences table does not exist on first launch, so the first save attempt would
      fail with a SQL 'no such table' error. The hydration guard in get_active_surface (CR-03)
      also contains a fragile default-value check that would silently fail to restore the Chat
      surface if Surface::default() changes.
    artifacts:
      - path: "src-tauri/src/main.rs"
        issue: "set_active_surface not reachable — ShellPreferenceStore unmanaged"
      - path: "src-tauri/capabilities/main.json"
        issue: "set_active_surface not in permissions — blocked before Rust handler"
      - path: "src-tauri/src/ipc/app_shell.rs"
        issue: "Hydration guard 'if current == Surface::Chat' (line 53) is a fragile default-value check — will silently skip DB restore for any future refactor that changes Surface::default()"
    missing:
      - "Fix main.rs and capabilities (see gap 1)"
      - "Fix SqlitePool::open() migrations (see gap 1)"
      - "Introduce a hydrated: bool flag in ShellState instead of default-value equality check"

human_verification:
  - test: "Launch the app with npm run dev (after gap fixes) and switch surfaces"
    expected: "Each surface (Chat, History, Settings, Artifacts) renders correctly, switching is immediate, no console errors from IPC"
    why_human: "Requires Tauri runtime, visual inspection of shell layout, and DevTools console monitoring"
  - test: "Keyboard-only navigation across the four surfaces"
    expected: "Arrow keys move focus within the rail, Enter/Space activate the focused surface and move focus to the panel, Home/End jump to first/last, focus ring visible on each button"
    why_human: "Requires runtime and keyboard interaction; code design is correct but at/wai-aria behavior can only be confirmed with actual AT or browser keyboard testing"
  - test: "Screen reader surface announcement on switch"
    expected: "Switching surface announces the surface name via the aria-live polite region; no duplicate 'application' role announcements from nested AppShell/WorkspaceShell"
    why_human: "Requires a screen reader (VoiceOver, NVDA, JAWS) to verify AT tree and live region firing"
  - test: "Restart surface persistence (after gap fixes)"
    expected: "Switch to Settings surface, quit the app, relaunch — shell opens to Settings, not Chat"
    why_human: "Requires Tauri runtime and actual process restart to verify SQLite persistence across sessions"
  - test: "cargo test --workspace --all-targets"
    expected: "All 5 integration tests in src-tauri/tests/app_shell.rs pass plus all inline unit tests"
    why_human: "Cargo/Rust not installed in verification environment; tests are structurally correct but were not executed"
---

# Phase 01: App Shell Verification Report

**Phase Goal:** Create the runnable desktop shell foundation for Phase 1 by establishing the Tauri v2 + Svelte 5/SvelteKit build stack, wiring the backend bootstrap, and adding backend-owned persistence for the initial active surface. Refine into an accessible, keyboard-navigable workspace switcher with smoke coverage.
**Verified:** 2026-06-13T20:00:00Z
**Status:** gaps_found
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The app opens into a usable desktop shell instead of a blank scaffold | FAILED | main.rs manages only AppState::default(); SqlitePool and ShellPreferenceStore are never registered; capabilities/main.json does not grant either IPC command; SqlitePool::open() never runs migrations |
| 2 | The shell loads a backend-owned active-surface preference on startup | FAILED | Three blockers compound: unmanaged state (CR-01), missing capability grant (CR-02), missing migrations in open() (CR-04) |
| 3 | Frontend navigation changes the visible surface without exposing raw file paths or secrets | VERIFIED (code) | surface.ts uses typed invoke(), optimistic rollback, no localStorage/sessionStorage; Surface type is a closed enum; IPC commands enforce window label; privacy boundary is correctly designed — but blocked at runtime by CR-01/CR-02 |
| 4 | The user can switch between chat, history, settings, and artifact surfaces from the shell | VERIFIED (code) | WorkspaceShell, SurfaceRail, SurfacePanel render all four surfaces; routing in +page.svelte is substantive; ARIA tablist/tab/tabpanel wiring is complete |
| 5 | Focus order and keyboard activation work for the surface switcher | VERIFIED (code) | SurfaceRail implements roving tabindex, Arrow/Home/End/Enter/Space handlers; activate() moves focus to #main-content; focus-visible CSS present |
| 6 | The active surface survives a restart through backend-owned persistence | FAILED | Depends on set_active_surface working (blocked by CR-01 + CR-02) and shell_preferences table existing (blocked by CR-04) |
| 7 | Backend smoke tests cover the write/read/restore path for the active surface | VERIFIED (structure) | src-tauri/tests/app_shell.rs has 5 named integration tests covering round-trip, fresh-db hydration, UPSERT overwrite, session restore, and all-surfaces persistence; tests use SqlitePool::from_connection() which bypasses the broken SqlitePool::open() path |

**Score: 3/7 truths verified** (truths 3, 4, 5 verified at code level; truths 1, 2, 6 failed due to runtime wiring blockers; truth 7 verified structurally but not executed)

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `package.json` | Build manifests with dev/build/check scripts | VERIFIED | Scripts present: dev (tauri dev), build (tauri build), check (svelte-kit sync + svelte-check), frontend:dev, frontend:build |
| `src-tauri/Cargo.toml` | Rust manifest with rusqlite, tokio, serde, tauri 2 | VERIFIED | All dependencies present including rusqlite bundled feature |
| `src-tauri/src/main.rs` | Thin bootstrap registering IPC commands | STUB | Commands registered in generate_handler! but state management is incomplete — no .setup() hook, SqlitePool and ShellPreferenceStore never created or managed |
| `src-tauri/src/ipc/app_shell.rs` | Typed get_active_surface and set_active_surface commands | VERIFIED (code) | Commands are substantive and correct in design; window label enforcement present; ShellError serializes with code field; blocked at runtime by missing .manage() calls |
| `src/routes/+layout.svelte` | Layout that calls surfaceStore.hydrate() on mount | VERIFIED | onMount(() => surfaceStore.hydrate()) present; note: promise is not awaited (WR-04) |
| `src/lib/components/AppShell.svelte` | Root layout with nav and main landmarks | VERIFIED | role="application", nav, main landmarks present; NOTE: role="application" conflicts with WorkspaceShell.svelte (WR-02) |
| `src/lib/stores/surface.ts` | Svelte 5 store with IPC hydration and no browser storage | VERIFIED | $state runes, invoke() for both commands, optimistic rollback, no localStorage |
| `src/lib/components/WorkspaceShell.svelte` | Composed layout with skip link, nav, main, StatusRegion | VERIFIED | All elements present; role="application" duplicates AppShell (WR-02) |
| `src/lib/components/SurfacePanel.svelte` | tabpanel wrapper for active surface | VERIFIED | role="tabpanel", aria-label={surfaceLabel}, id="surface-panel" |
| `src/lib/components/StatusRegion.svelte` | aria-live polite status region | VERIFIED | role="status", aria-live="polite", derives announceText from error/loading/message |
| `src-tauri/tests/app_shell.rs` | Backend integration tests for shell preference persistence | VERIFIED | 5 named tests covering required behaviors; structurally complete |
| `src-tauri/capabilities/main.json` | Capability grant including shell IPC commands | FAILED | get_active_surface and set_active_surface absent from permissions array |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `surface.ts hydrate()` | `get_active_surface` Rust command | `invoke('get_active_surface')` | NOT_WIRED at runtime | Frontend call is correct; blocked by missing capability grant (CR-02) and missing .manage() (CR-01) |
| `surface.ts setSurface()` | `set_active_surface` Rust command | `invoke('set_active_surface', { surface })` | NOT_WIRED at runtime | Same blockers as above |
| `ipc/app_shell.rs get_active_surface` | `ShellPreferenceStore` | `tauri::State<'_, ShellPreferenceStore>` | NOT_WIRED | ShellPreferenceStore never registered with .manage(); Tauri 2 will panic on injection |
| `ipc/app_shell.rs set_active_surface` | `ShellPreferenceStore` | `tauri::State<'_, ShellPreferenceStore>` | NOT_WIRED | Same as above |
| `ShellPreferenceStore.save_active_surface` | `shell_preferences` SQL table | `INSERT INTO shell_preferences ...` | NOT_WIRED at runtime | Table does not exist — SqlitePool::open() never calls run_migrations (CR-04) |
| `ShellPreferenceStore.load_active_surface` | `shell_preferences` SQL table | `SELECT active_surface FROM shell_preferences` | NOT_WIRED at runtime | Same — table absent on first launch |
| `SurfaceRail tab buttons` | `SurfacePanel#surface-panel` | `aria-controls="surface-panel"` | WIRED | aria-controls present on each tab button; panel id matches |
| `+layout.svelte onMount` | `surfaceStore.hydrate()` | `onMount(() => surfaceStore.hydrate())` | PARTIAL | Call is present but promise is not awaited or .catch()-ed (WR-04) |
| `+page.svelte` | `WorkspaceShell > SurfacePanel > {surface component}` | `$derived(surfaceStore.surface)` reactive routing | WIRED | All four surface components rendered conditionally; derivation is reactive |

---

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|-------------------|--------|
| `surface.ts` | `surface` ($state) | `invoke('get_active_surface')` in `hydrate()` | No — IPC call blocked at runtime by CR-01 + CR-02; fallback is hardcoded 'chat' | HOLLOW at runtime (falls back to static default) |
| `StatusRegion.svelte` | `announceText` | `message`, `loading`, `error` props from surfaceStore | Derives from surface.ts state — correctly reactive when store works | STATIC at runtime (shell always shows "Chat surface active" due to fallback) |
| `SurfaceRail.svelte` | `activeSurface` | `$derived(surfaceStore.surface)` | Reactive from store | STATIC at runtime (always 'chat' fallback) |

---

## Behavioral Spot-Checks

Step 7b: SKIPPED for Rust/Tauri runtime checks — Tauri desktop app requires a native runtime environment to invoke. `npm run check` was verified passing by executor (275 files, 0 errors, 0 warnings) but not re-run in this environment.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Frontend typecheck | `npm run check` | 275 files, 0 errors (executor-reported) | SKIP — not re-runnable without npm environment |
| Rust unit/integration tests | `cargo test --workspace --all-targets` | NOT RUN by executor | SKIP — Cargo not installed in verification environment |
| App launches and hydrates | `npm run dev` | NOT RUN by executor | SKIP — requires Tauri desktop runtime |

---

## Probe Execution

No `scripts/*/tests/probe-*.sh` files were declared in either PLAN or found in the repository.

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SHELL-01 | 01-01-PLAN.md | User can launch the desktop app and reach the main workspace | BLOCKED | App cannot launch correctly due to CR-01 (unmanaged state panic), CR-02 (capability missing), CR-04 (no migrations). Frontend scaffold is complete but backend wiring prevents runtime operation. |
| SHELL-02 | 01-02-PLAN.md | User can navigate between chat, history, settings, and artifact surfaces | PARTIAL | Surface navigation code is complete and accessible (tablist/tab/tabpanel, keyboard, aria-live). Surface switching is blocked at the IPC persistence layer (CR-01, CR-02), but falls back gracefully to frontend-local state in error mode. Visual navigation works; persisted navigation is broken. |

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src-tauri/src/main.rs` | 23 | `.manage(AppState::default())` only — ShellPreferenceStore and SqlitePool unmanaged | BLOCKER | Both IPC commands panic at runtime when Tauri attempts to inject unmanaged state |
| `src-tauri/capabilities/main.json` | 6-11 | `permissions` array missing get_active_surface and set_active_surface | BLOCKER | Tauri 2 deny-by-default system rejects both commands before Rust handler is reached |
| `src-tauri/src/storage/sqlite.rs` | 25-38 | `SqlitePool::open()` does not call `run_migrations` despite doc-comment contract | BLOCKER | shell_preferences table absent on first launch; save and load both fail with "no such table" |
| `src-tauri/src/ipc/app_shell.rs` | 53 | `if current == Surface::Chat` hydration guard uses default-value equality instead of hydrated flag | WARNING | Silently fails to restore Chat explicitly if Surface::default() ever changes; semantics are fragile |
| `src/lib/components/AppShell.svelte` | 21 | `role="application"` duplicated in parent layout | WARNING | AppShell and WorkspaceShell are both active in the DOM hierarchy; nested role="application" is invalid ARIA |
| `src/lib/components/SurfaceRail.svelte` | 31-32 | `surfaces.findIndex(...) \|\| 0` — falsy coercion treats index 0 (Chat) the same as -1 (not found) | WARNING | Correct by coincidence; breaks if Chat moves from position 0 or pattern is copied |
| `src/routes/+layout.svelte` | 10 | `surfaceStore.hydrate()` return value not awaited — floating promise | WARNING | Unhandled rejections from synchronous throws before the try block are silently dropped |

---

## Human Verification Required

The following items require a runtime environment or assistive technology that cannot be verified programmatically from the source tree. These are contingent on the three blocker gaps being resolved first.

### 1. App Launch and Surface Display

**Test:** After fixing CR-01, CR-02, and CR-04, run `npm install && npm run dev` and observe the launched app.
**Expected:** App opens into the workspace shell showing the SurfaceRail on the left with four labeled tabs (Chat, History, Settings, Artifacts) and the Chat surface as the default content area. No runtime panics, no IPC error messages in the StatusRegion on first launch.
**Why human:** Requires Tauri runtime (native window), visual inspection, and DevTools console check.

### 2. Keyboard-Only Surface Navigation

**Test:** With the app running, close any pointer device and navigate using only Tab, Arrow keys, Enter, Space, Home, End.
**Expected:** Tab moves to the skip-to-content link, then to the rail, then into the panel. Arrow Up/Down move focus between rail tabs. Enter/Space activate the focused tab and move focus to `#main-content`. Home jumps to Chat tab, End jumps to Artifacts tab. Focus ring (2px blue outline) visible on every focused element.
**Why human:** ARIA APG keyboard interaction can only be confirmed through actual keyboard use in a running browser/WebView.

### 3. Screen Reader Surface Announcements

**Test:** With a screen reader (VoiceOver/macOS, NVDA/Windows, or Orca/Linux) active, switch surfaces using keyboard navigation.
**Expected:** Screen reader announces the surface name when focus moves to a tab (via aria-label), announces "selected" state change when a tab is activated (via aria-selected), and announces the surface change non-interruptively via the StatusRegion's aria-live="polite" region. No duplicate "application" landmark announcements from the nested AppShell/WorkspaceShell structure.
**Why human:** AT behavior depends on specific screen reader implementations and browser/WebView combination.

### 4. Surface Persistence Across Restarts

**Test:** After fixing all gaps, switch to the History surface, quit the app, relaunch.
**Expected:** The app opens to the History surface (not Chat), confirming SQLite persistence round-trip through the full IPC stack.
**Why human:** Requires actual process lifecycle (quit + relaunch) and visual inspection of restored state.

### 5. cargo test Execution

**Test:** In an environment with Rust/Cargo installed, run `cargo test --workspace --all-targets` from the repository root.
**Expected:** All 5 integration tests in `src-tauri/tests/app_shell.rs` pass, plus all inline unit tests in `app_state.rs`, `storage/sqlite.rs`, `storage/migrations.rs`, and `ipc/app_shell.rs`. No test failures or panics.
**Why human:** Cargo not installed in verification environment. Tests are structurally correct but were not executed by the executor or this verifier.

---

## Gaps Summary

Three gaps all stem from the same root cause: **the Tauri builder setup is incomplete**. The executor built correct Rust storage and IPC modules in isolation, and correct frontend components, but failed to wire them together in `main.rs` (the integration point) and failed to grant IPC access in `capabilities/main.json`. A single focused plan that adds a `.setup()` hook to `main.rs` and updates `capabilities/main.json` would close all three blockers simultaneously, and simultaneously fix CR-04 either by calling `run_migrations` inside `setup()` or inside `SqlitePool::open()`.

**Gap 1 (root cause):** `main.rs` has no `.setup()` hook. No path exists from the Tauri app startup to `SqlitePool::open()` → `run_migrations()` → `app.manage(pool)` → `app.manage(store)`. Without this, the entire backend persistence chain is structurally disconnected.

**Gap 2 (access control):** `capabilities/main.json` does not grant the two shell IPC commands. Even if Gap 1 were fixed, the frontend invoke() calls would be rejected by Tauri's permission layer.

**Gap 3 (secondary):** `SqlitePool::open()` doc-comment promises migration but the implementation omits the call. This must be fixed in the same pass as Gap 1, or the setup hook must call `run_migrations` explicitly.

The two secondary issues (CR-03 hydration guard fragility, WR-02 duplicate `role="application"`) are correctness risks that should be fixed but do not prevent demo-level operation once the three blockers are resolved.

---

_Verified: 2026-06-13T20:00:00Z_
_Verifier: Claude (gsd-verifier)_
