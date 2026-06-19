# External Integrations

**Analysis Date:** 2026-06-13

## APIs & External Services

**AI Providers (planned, not yet implemented):**

- OpenRouter — intended as the primary AI provider gateway
  - Module: `src-tauri/src/providers/openrouter.rs` (scaffold placeholder)
  - Routing: `src-tauri/src/providers/routing.rs` (scaffold placeholder)
  - Capability detection: `src-tauri/src/providers/capabilities.rs` (scaffold placeholder)
  - SSE streaming transport: `src-tauri/src/providers/sse.rs` (scaffold placeholder)
  - Auth: Backend-owned credential handling via `src-tauri/src/security/secrets.rs` (scaffold placeholder)
  - Provider selection is deterministic and backend-owned; the frontend never receives provider credentials or routing decisions

**No other external APIs are wired at this time.** Provider modules are module-level placeholders. The IPC commands for provider management (`src-tauri/src/ipc/providers.rs`) are also scaffold stubs.

## Data Storage

**Databases:**

- SQLite (bundled via `rusqlite 0.31` `bundled` feature)
  - Connection: Mutex-guarded single connection in `src-tauri/src/storage/sqlite.rs` (`SqlitePool`)
  - Location: resolved at startup via `app.path().app_data_dir()` in `src-tauri/src/main.rs`; path: `<app_data>/desktop-ai-client.db`
  - WAL mode, `synchronous = NORMAL`, `foreign_keys = ON`, `busy_timeout = 5000` enforced on every connection
  - Migrations: `src-tauri/src/storage/migrations.rs`; current migration count: 1 (`0001` — `shell_preferences` table)
  - Domain store: `ShellPreferenceStore` in `src-tauri/src/storage/sqlite.rs`; the only layer permitted to read/write the `shell_preferences` table

**File Storage:**

- Local filesystem only; opaque token model planned via `src-tauri/src/security/file_tokens.rs` (scaffold placeholder)
- Raw file paths are never exposed to the frontend renderer

**Caching:**

- None implemented; in-memory shell state (`AppState` / `ShellState`) serves as a hydration cache to avoid redundant SQLite reads within a session

**FTS / Search:**

- `src-tauri/src/storage/fts.rs` — scaffold placeholder for SQLite FTS5 full-text search

**Backup / Retention:**

- `src-tauri/src/storage/backup.rs` — scaffold placeholder
- `src-tauri/src/storage/retention.rs` — scaffold placeholder

## Tauri IPC Commands

All registered IPC commands are listed in `src-tauri/src/main.rs` inside `tauri::generate_handler![...]`. The capability grant file `src-tauri/capabilities/main.json` enforces a deny-by-default allowlist for the `main` window.

**Currently registered and callable from the frontend:**

| Command              | Module                           | Capability                 | Sensitivity |
| -------------------- | -------------------------------- | -------------------------- | ----------- |
| `get_active_surface` | `src-tauri/src/ipc/app_shell.rs` | `allow-get-active-surface` | low         |
| `set_active_surface` | `src-tauri/src/ipc/app_shell.rs` | `allow-set-active-surface` | low         |

Both commands validate the caller window label (`"main"`) as backend-side enforcement; capability grants are defense-in-depth.

**Scaffolded IPC modules (not yet registered):**

| Module      | File                             | Status      |
| ----------- | -------------------------------- | ----------- |
| `chat`      | `src-tauri/src/ipc/chat.rs`      | placeholder |
| `files`     | `src-tauri/src/ipc/files.rs`     | placeholder |
| `history`   | `src-tauri/src/ipc/history.rs`   | placeholder |
| `inventory` | `src-tauri/src/ipc/inventory.rs` | placeholder |
| `privacy`   | `src-tauri/src/ipc/privacy.rs`   | placeholder |
| `providers` | `src-tauri/src/ipc/providers.rs` | placeholder |

## Authentication & Identity

**Auth Provider:**

- None implemented. Provider credentials are intended to be backend-owned secrets, managed via `src-tauri/src/security/secrets.rs` (placeholder).
- No user authentication layer exists yet.
- The security module defines the boundary: `src-tauri/src/security/` contains `secrets.rs`, `file_tokens.rs`, `artifact_sandbox.rs`, `command_policy.rs`, and `redaction.rs` — all currently placeholders.

## Monitoring & Observability

**Telemetry:**

- `src-tauri/src/telemetry/audit_log.rs` — scaffold placeholder for audit log
- `src-tauri/src/telemetry/release_evidence.rs` — scaffold placeholder for release evidence capture
- `log 0.4` facade wired in `src-tauri/Cargo.toml`; concrete backend (e.g. `env_logger`) not yet registered
- Privacy rule: telemetry must never contain secrets, raw paths, or prompt payloads

**Error Tracking:**

- None wired. Errors surface as typed IPC error enums serialized to the frontend (e.g. `ShellError` in `src-tauri/src/ipc/app_shell.rs`).

## Tauri Plugins

| Plugin                | Version | Purpose                                                                     |
| --------------------- | ------- | --------------------------------------------------------------------------- |
| `tauri-plugin-opener` | `2`     | Opens external URLs from the backend; not exposed as a raw frontend command |

## Tauri Security Configuration

- `withGlobalTauri: false` — frontend must explicitly import `@tauri-apps/api`; no global `window.__TAURI__` pollution
- CSP (from `src-tauri/tauri.conf.json`): `default-src 'self'; connect-src ipc: http://ipc.localhost; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:`
- Single window: label `"main"`, 1280×900, minimum 800×600

## CI/CD & Deployment

**Hosting:**

- Native desktop application; bundled via `tauri build` for all platforms (`"targets": "all"`)

**CI Pipeline:**

- Not detected (no `.github/workflows/` CI config observed at time of analysis)

## Webhooks & Callbacks

**Incoming:** None
**Outgoing:** None currently wired; SSE streaming from AI providers is planned via `src-tauri/src/providers/sse.rs`

---

_Integration audit: 2026-06-13_
