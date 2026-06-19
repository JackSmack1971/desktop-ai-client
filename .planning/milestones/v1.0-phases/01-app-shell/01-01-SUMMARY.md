---
phase: '01-app-shell'
plan: '01'
subsystem: 'app-shell'
tags:
  - tauri-v2
  - svelte5
  - sveltekit
  - sqlite
  - ipc
  - shell-persistence
dependency_graph:
  requires: []
  provides:
    - runnable-app-scaffold
    - backend-owned-shell-ipc
    - shell-preference-sqlite
    - surface-rail-navigation
  affects:
    - src-tauri/src/ipc
    - src-tauri/src/storage
    - src/lib/stores
    - src/lib/components
tech_stack:
  added:
    - '@sveltejs/kit ^2.0.0 — SvelteKit SSG adapter-static for Tauri frontend'
    - '@sveltejs/vite-plugin-svelte ^5.0.0 — Svelte 5 Vite plugin'
    - '@sveltejs/adapter-static ^3.0.0 — builds static dist/ for Tauri frontendDist'
    - 'svelte ^5.0.0 — Svelte 5 with runes'
    - 'vite ^6.0.0 — build tool, port 1420 for Tauri dev'
    - 'svelte-check ^4.0.0 — frontend type checker'
    - 'typescript ^5.0.0'
    - '@tauri-apps/api ^2.0.0 — typed IPC invoke from frontend'
    - '@tauri-apps/cli ^2.0.0 — tauri dev / tauri build'
    - 'rusqlite 0.31 (bundled) — SQLite with WAL, foreign keys, busy_timeout'
    - 'serde + serde_json — IPC serialization'
    - 'thiserror — typed error enums for IPC'
    - 'uuid 1 — stream IDs (future)'
    - 'chrono 0.4 — timestamps'
    - 'tokio 1 rt-multi-thread — async runtime'
    - 'tauri 2 / tauri-build 2 — Tauri v2 framework'
  patterns:
    - 'Svelte 5 $state/$derived runes for reactive surface state'
    - 'Backend-owned IPC commands with window-label caller assertion'
    - 'Optimistic frontend updates with rollback on IPC failure'
    - 'rusqlite WAL mode + busy_timeout on all connections'
    - 'Transactional migration runner with schema_migrations tracking table'
    - 'Deny-by-inventory capability model (capabilities/main.json)'
key_files:
  created:
    - package.json
    - Cargo.toml
    - svelte.config.js
    - vite.config.ts
    - tsconfig.json
    - .gitignore
    - src-tauri/Cargo.toml
    - src-tauri/build.rs
    - src-tauri/capabilities/main.json
    - src-tauri/src/lib.rs
    - src-tauri/src/ipc/app_shell.rs
    - src-tauri/src/storage/sqlite.rs
    - src-tauri/src/storage/migrations.rs
    - src/lib/stores/surface.ts
    - src/lib/components/AppShell.svelte
    - src/lib/components/SurfaceRail.svelte
    - src/lib/components/surfaces/ChatSurface.svelte
    - src/lib/components/surfaces/HistorySurface.svelte
    - src/lib/components/surfaces/SettingsSurface.svelte
    - src/lib/components/surfaces/ArtifactsSurface.svelte
    - src/routes/+layout.svelte
    - src/routes/+page.svelte
  modified:
    - src-tauri/tauri.conf.json
    - src-tauri/src/main.rs
    - src-tauri/src/app_state.rs
    - src-tauri/src/ipc/mod.rs
    - src-tauri/src/storage/mod.rs
    - src/app.html
decisions:
  - 'Used @sveltejs/vite-plugin-svelte@5 (not v4) because v4 requires Vite ^5, while v5 correctly targets Vite ^6'
  - 'Import sveltekit() from @sveltejs/kit/vite, not from @sveltejs/vite-plugin-svelte (only re-exports svelte/vitePreprocess)'
  - 'Replaced deprecated csrf.checkOrigin with csrf.trustedOrigins listing Tauri localhost origins'
  - 'Added @types/node as devDependency for process.env references in vite.config.ts'
  - 'Bundled SQLite via rusqlite bundled feature to avoid system library dependency in CI'
  - 'Surface enum in app_state.rs uses serde rename_all=snake_case to match frontend Surface type literals'
  - 'ShellPreferenceStore uses UPSERT (INSERT ON CONFLICT DO UPDATE) to avoid race between insert/update'
  - 'Migration runner uses SAVEPOINTs not outer transactions because SQLite forbids DDL in some transaction modes'
metrics:
  duration: '636s (10m 36s)'
  completed_date: '2026-06-13'
  completed_tasks: 2
  total_tasks: 2
  files_created: 22
  files_modified: 6
---

# Phase 01 Plan 01: App Shell Bootstrap Summary

**One-liner:** Tauri v2 + Svelte 5/SvelteKit desktop shell with backend-owned SQLite surface-preference persistence via typed IPC commands.

## What Was Built

### Task 1 — Bootstrap the runnable app stack

Established the complete buildable frontend and Rust manifests:

- `package.json` with `npm run dev` (tauri dev), `npm run build` (tauri build), `npm run check` (svelte-kit sync + svelte-check), `npm run frontend:dev`, and `npm run frontend:build` scripts.
- `svelte.config.js` using `@sveltejs/adapter-static` targeting `../dist` (aligned with `tauri.conf.json` `frontendDist`). CSRF trustedOrigins set for Tauri localhost variants.
- `vite.config.ts` importing `sveltekit()` from `@sveltejs/kit/vite`, dev server on port 1420 (`strictPort: true`), per-platform build targets (Chrome 105 / Safari 13 / Firefox 115), source maps only in debug builds.
- `tsconfig.json` extending `.svelte-kit/tsconfig.json` with strict mode and `@types/node`.
- Root `Cargo.toml` workspace manifest with `src-tauri` member.
- `src-tauri/Cargo.toml` with `rusqlite` (bundled), `tokio`, `serde`, `thiserror`, `tauri 2`, release profile with LTO/size/strip/abort-on-panic.
- `src-tauri/build.rs` calling `tauri_build::build()`.
- `src-tauri/tauri.conf.json` with `withGlobalTauri: false`, labeled `main` window, `devUrl: http://localhost:1420`, `beforeDevCommand: npm run frontend:dev`, explicit CSP header.
- `src-tauri/capabilities/main.json` deny-by-inventory capability for the main window.
- `src-tauri/src/main.rs` thin bootstrap: registers `get_active_surface` and `set_active_surface` via `tauri::generate_handler![]` with `#[cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.
- `src-tauri/src/lib.rs` library crate entry for `cargo test`.
- `src-tauri/src/app_state.rs` with typed `Surface` enum (Chat/History/Settings/Artifacts), serde snake_case serialization, `FromStr`/`Display` impls, `ShellState`, and unit tests.

**Verification:** `npm run check` passes: 272 files, 0 errors, 0 warnings.

### Task 2 — Add backend-owned shell persistence

Implemented typed IPC and SQLite-backed shell preference storage:

- `src-tauri/src/ipc/app_shell.rs`: `get_active_surface` and `set_active_surface` Tauri commands. Both assert caller window label is `main` (backend enforcement supplementing capability files). `ShellError` serializes with `code` field in SCREAMING_SNAKE_CASE.
- `src-tauri/src/storage/sqlite.rs`: `SqlitePool` with WAL mode, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`. `ShellPreferenceStore` domain API with `save_active_surface` (UPSERT) and `load_active_surface`. Unit tests verify round-trip save/load on an in-memory database.
- `src-tauri/src/storage/migrations.rs`: Transactional migration runner using SAVEPOINTs, `schema_migrations` tracking table, two migrations (0001: tracking table; 0002: `shell_preferences` table). Unit tests: fresh-db applies all migrations, re-run is idempotent, table is writable after migration.
- `src/lib/stores/surface.ts`: Svelte 5 `$state`-based store. `hydrate()` calls `get_active_surface` IPC on layout mount. `setSurface()` does optimistic update then `set_active_surface` IPC, rolling back on failure. Never reads/writes browser storage.
- `src/lib/components/AppShell.svelte`: Root layout with `role="application"` and `<nav>`/`<main>` ARIA landmarks.
- `src/lib/components/SurfaceRail.svelte`: Icon nav rail with `aria-current`, `aria-label`, `aria-live`, and `focus-visible` CSS indicators meeting the accessibility release gate baseline.
- Four surface scaffold components (Chat, History, Settings, Artifacts) with `role="region"` and `aria-label`.
- `src/routes/+layout.svelte`: calls `surfaceStore.hydrate()` in `onMount`.
- `src/routes/+page.svelte`: routes to surface components via `$derived(surfaceStore.surface)`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Wrong @sveltejs/vite-plugin-svelte version specified**

- **Found during:** Task 1 — `npm install` failed with ERESOLVE
- **Issue:** Package version `^4.0.0` requires Vite `^5.0.0`; project uses Vite `^6.0.0`. Version `^5.0.0` is the correct peer for Vite 6.
- **Fix:** Updated `package.json` to `@sveltejs/vite-plugin-svelte: "^5.0.0"`.
- **Files modified:** `package.json`

**2. [Rule 1 - Bug] Incorrect vitekit import path in vite.config.ts**

- **Found during:** Task 1 — `npm run check` failed with "no exported member named 'sveltekit'"
- **Issue:** `@sveltejs/vite-plugin-svelte@5` exports only `svelte` and `vitePreprocess`. The `sveltekit()` plugin function is re-exported from `@sveltejs/kit/vite`.
- **Fix:** Changed import to `import { sveltekit } from '@sveltejs/kit/vite'`.
- **Files modified:** `vite.config.ts`

**3. [Rule 2 - Missing functionality] Added @types/node for process.env**

- **Found during:** Task 1 — svelte-check errors on `process.env` in vite.config.ts
- **Fix:** Added `@types/node` devDependency; added `"types": ["@types/node"]` to `tsconfig.json`.
- **Files modified:** `package.json`, `tsconfig.json`

**4. [Rule 1 - Bug] Deprecated csrf.checkOrigin in svelte.config.js**

- **Found during:** Task 1 — svelte-kit warned about deprecated `csrf.checkOrigin` option
- **Fix:** Replaced with `csrf.trustedOrigins` listing Tauri localhost variants.
- **Files modified:** `svelte.config.js`

## Known Stubs

The four surface components (ChatSurface, HistorySurface, SettingsSurface, ArtifactsSurface) render labeled placeholder text. These are intentional scaffolds — they prove the shell routing works and that each surface has a named entry point. Full conversation UI, search, settings, and artifact editor are in-scope for later plans in Phase 01.

The pre-existing Rust scaffold files (`ipc/chat.rs`, `ipc/history.rs`, etc.) remain as placeholders from the original codebase scaffold; they are not part of this plan's deliverables.

## Verification Results

| Check                                  | Status  | Notes                                                                                                                                                                                                        |
| -------------------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `npm run check`                        | PASSED  | 272 files, 0 errors, 0 warnings                                                                                                                                                                              |
| `cargo test --workspace --all-targets` | NOT RUN | Rust/Cargo not installed in execution environment. Backend code (app_state.rs, storage/sqlite.rs, storage/migrations.rs, ipc/app_shell.rs) contains inline unit tests that will run when Cargo is available. |
| App launches and restores surface      | NOT RUN | Requires Tauri runtime environment; manual verification step for developer after `npm install && npm run dev`                                                                                                |

## Threat Flags

No new threat surface outside the plan's intended IPC boundary was introduced. The two new IPC commands (`get_active_surface`, `set_active_surface`) are:

- Scoped to the `main` window label by both capability grant and backend assertion
- Read/write only the `active_surface` preference — no secrets, no file paths, no prompt content
- Typed with non-extensible enums (Surface must match a known variant)

## Self-Check: PASSED

Files verified present:

- package.json: FOUND
- src-tauri/Cargo.toml: FOUND
- src-tauri/tauri.conf.json: FOUND
- src-tauri/src/main.rs: FOUND
- src-tauri/src/ipc/app_shell.rs: FOUND
- src/routes/+layout.svelte: FOUND
- src/lib/components/AppShell.svelte: FOUND
- src/lib/stores/surface.ts: FOUND

Commits verified:

- a4cd544: feat(01-01): bootstrap the runnable app stack
- 5719a9c: feat(01-01): add backend-owned shell persistence
