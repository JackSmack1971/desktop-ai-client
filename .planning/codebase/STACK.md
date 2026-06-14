# Technology Stack

**Analysis Date:** 2026-06-13

## Languages

**Primary:**
- Rust 1.77 (minimum) — Tauri backend, all business logic, IPC, storage, security, telemetry (`src-tauri/`)
- TypeScript 5.x — SvelteKit frontend (`src/`)

**Secondary:**
- SQL (SQLite dialect) — schema migrations and queries (`src-tauri/src/storage/migrations.rs`, `src-tauri/src/storage/sqlite.rs`)

## Runtime

**Environment:**
- Desktop: Tauri 2.x runtime embedding a platform WebView (Chromium on Windows, WebKit on macOS, Gecko on Linux)
- No Node.js server at runtime; Node is build-time only

**Package Manager:**
- npm (no `.nvmrc` or pinned Node version detected)

## Frameworks

**Core:**
- Tauri 2.x (`tauri = "2"`, `@tauri-apps/cli ^2.0.0`, `@tauri-apps/api ^2.0.0`) — desktop shell, IPC bridge, capability model, plugin system
- SvelteKit 2.x (`@sveltejs/kit ^2.0.0`) — frontend routing and static adapter
- Svelte 5.x (`svelte ^5.0.0`) — reactive UI components

**Build/Dev:**
- Vite 6.x (`vite ^6.0.0`) — frontend bundler, dev server on port 1420
- `@sveltejs/adapter-static ^3.0.0` — outputs static files to `../dist` for Tauri to serve
- `@sveltejs/vite-plugin-svelte ^5.0.0` — Vite/Svelte integration
- `tauri-build = "2"` — Rust build script for Tauri code generation
- `svelte-check ^4.0.0` — TypeScript/Svelte type checking (`npm run check`)

**Testing:**
- Rust built-in `#[test]` / `#[cfg(test)]` — no external Rust test runner; tests embedded inline in modules
- No frontend test runner detected (Vitest/Jest absent from `package.json`)

## Key Dependencies

**Critical (Rust):**
- `rusqlite 0.31` with `bundled` feature — SQLite compiled into the binary, no system SQLite dependency (`src-tauri/Cargo.toml`)
- `tokio 1` (`rt-multi-thread`, `macros`) — async runtime for IPC command handlers
- `serde 1` + `serde_json 1` — serialization of all IPC types crossing the Tauri boundary
- `thiserror 1` — typed error derivation for IPC error enums (e.g. `ShellError` in `src-tauri/src/ipc/app_shell.rs`)
- `tauri-plugin-opener 2` — backend-side opening of external URLs

**Supporting (Rust):**
- `uuid 1` (`v4`) — UUID generation (modules scaffolded, not yet actively used)
- `chrono 0.4` (`serde`) — timestamp handling with serde support
- `log 0.4` — structured logging facade (concrete implementation not yet wired)

**Frontend:**
- `@tauri-apps/api ^2.0.0` — Tauri IPC invoke and event APIs for the frontend

## Configuration

**Environment:**
- Frontend env vars exposed via Vite prefix `VITE_` and `TAURI_ENV_`; secrets must never use these prefixes
- `TAURI_DEV_HOST` controls Vite dev server bind address (defaults to `localhost`)
- `TAURI_ENV_PLATFORM` and `TAURI_ENV_DEBUG` drive per-platform build targets and source maps in `vite.config.ts`
- No `.env` files committed; provider credential env var names not yet defined (provider modules are scaffold placeholders)

**Build:**
- `vite.config.ts` — frontend bundler config; per-platform browser targets; source maps in debug only
- `svelte.config.js` — SvelteKit config; static adapter; CSRF trusted origins for Tauri URLs
- `src-tauri/tauri.conf.json` — app metadata, window definition, CSP, bundle targets
- `src-tauri/Cargo.toml` — Rust manifest; release profile: LTO, size-optimize, strip symbols, `panic = "abort"`, single codegen unit

## Release Profile (Rust)

Defined in `src-tauri/Cargo.toml` under `[profile.release]`:
- `lto = true` — link-time optimization
- `opt-level = "z"` — minimize binary size
- `strip = true` — strip debug symbols
- `panic = "abort"` — no stack unwinding in production
- `codegen-units = 1` — maximum optimization at cost of build speed

## Platform Requirements

**Development:**
- Rust toolchain >= 1.77 (enforced via `rust-version` in `src-tauri/Cargo.toml`; no `rust-toolchain.toml`)
- Node.js (version unpinned; no `.nvmrc`)
- Tauri CLI v2 (npm dev dependency: `@tauri-apps/cli ^2.0.0`)

**Production:**
- Bundled as a native desktop app via `tauri build`; targets all platforms (`"targets": "all"`)
- SQLite bundled into binary via `rusqlite` `bundled` feature — no runtime SQLite dependency
- Database stored at OS app-data directory resolved at startup by `app.path().app_data_dir()` in `src-tauri/src/main.rs`

---

*Stack analysis: 2026-06-13*
