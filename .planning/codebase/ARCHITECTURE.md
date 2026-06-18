<!-- refreshed: 2026-06-18 -->
# Architecture

**Analysis Date:** 2026-06-18

## System Overview

```text
┌──────────────────────────────────────────────────────────────────┐
│              Svelte 5 Renderer (WebView / main window)           │
│  `src/routes/`  `src/lib/components/`  `src/lib/stores/`         │
│  - No browser storage for app state                              │
│  - No raw file paths, secrets, or provider credentials           │
│  - Communicates exclusively via typed Tauri IPC commands         │
└────────────────────────┬─────────────────────────────────────────┘
                         │  Tauri IPC (@tauri-apps/api/core invoke)
                         ▼
┌──────────────────────────────────────────────────────────────────┐
│           Tauri Command Surface  `src-tauri/src/ipc/`            │
│  - Validates caller window label + command name via              │
│    security::command_policy::policy_check                       │
│  - Validates input at the IPC boundary                            │
│  - Returns structured, typed results                              │
│  - Never exposes secrets, raw paths, or arbitrary SQL            │
├──────────┬───────────┬────────────┬───────────┬──────────────────┤
│providers/│ security/ │  storage/  │telemetry/ │    app_state     │
│          │           │            │           │                  │
│Capability│ Secrets   │  SQLite    │ Audit log │ AppState /       │
│detection │ Redaction │  WAL pool  │ Release   │ ShellState       │
│Routing   │ File      │  Migrations│ evidence  │ (Mutex-guarded)  │
│OpenRouter│ tokens    │  FTS, Ret. │           │                  │
│SSE stream│ Cmd policy│  Backup    │           │                  │
│          │ Sandbox   │            │           │                  │
└──────────┴───────────┴──────────┬─┴───────────┴──────────────────┘
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────┐
│  OS Layer: App data dir, SQLite file, OS keychain (future)       │
│  `~/<app-data>/desktop-ai-client.db`                             │
└──────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | Key Files |
|-----------|----------------|-----------|
| IPC surface | 15 typed Tauri commands callable from the renderer; window-label and command-name enforcement | `src-tauri/src/ipc/*.rs` |
| Command policy | Single authority for "is this command callable from this window" | `src-tauri/src/security/command_policy.rs` |
| AppState | In-memory runtime state (`ShellState`, active surface, in-flight requests, secrets, file tokens); `Send + Sync` singleton | `src-tauri/src/app_state.rs` |
| providers | Capability detection, provider routing, OpenRouter transport, SSE streaming | `src-tauri/src/providers/` |
| security | Secrets store, file-access tokens, redaction, command policy, artifact sandbox | `src-tauri/src/security/` |
| storage | SQLite pool (WAL), typed domain stores, migration runner, FTS, retention, artifact persistence | `src-tauri/src/storage/` |
| telemetry | Audit log, release evidence capture (redaction-gated) | `src-tauri/src/telemetry/` |
| Svelte renderer | UI surfaces (chat, history, settings, artifacts), accessibility, surface navigation | `src/lib/components/`, `src/routes/` |
| Frontend stores | Typed stores bridging IPC and Svelte 5 reactive state | `src/lib/stores/*.ts` |

## Pattern Overview

**Overall:** Layered Tauri/Svelte desktop client with strict process-boundary privacy enforcement; the IPC command surface is fully implemented (15 registered commands across 7 domains), not a scaffold.

**Key Characteristics:**
- Backend owns all persistence, provider credentials, file authority, and routing decisions
- Renderer is treated as a potentially hostile surface — no secrets, raw paths, or SQL cross the IPC boundary
- Every IPC command validates window label and command-name membership through `security::command_policy::policy_check` before doing any work
- `ipc::inventory::verify_inventory()` cross-checks the command set against `security/command-inventory.toml`, registered handlers, permission files, capability files, release capabilities, and `command_policy`'s own table — six sources of truth, reconciled by `cargo run --bin verify-command-inventory`
- Svelte 5 runes (`$state`, `$derived`) for reactive frontend state; no `localStorage` or `sessionStorage`

## Layers

**Renderer Layer:**
- Purpose: Render UI surfaces and mediate user intent into IPC calls
- Location: `src/`
- Contains: SvelteKit routes, Svelte 5 components (`ChatSurface`, `HistorySurface`, `SettingsSurface`, `ArtifactsSurface`, plus shared `chat/`, `history/` component groups), typed stores, accessibility helpers
- Depends on: `@tauri-apps/api/core` for `invoke`; no direct OS or storage access
- Used by: end user via the Tauri WebView window

**IPC Command Layer:**
- Purpose: Validate and dispatch renderer requests to backend modules
- Location: `src-tauri/src/ipc/`
- Contains: `app_shell` (113 lines), `chat` (723 lines — streaming, cancellation, storage wiring, artifact detection), `history` (236 lines), `artifacts` (100 lines), `privacy` (115 lines), `files` (155 lines), `inventory` (726 lines — the cross-source verifier); `providers.rs` is a 1-line unregistered stub
- Depends on: `app_state`, `security`, `storage`, `providers`, `telemetry`
- Used by: renderer only, via Tauri `invoke`

**Business Logic Modules:**
- Purpose: Implement backend-owned concerns; each module owns exactly one concern
- Location: `src-tauri/src/{providers,security,storage,telemetry}/`
- Contains: `providers::routing` (105 lines), `providers::openrouter` (138 lines), `providers::sse` (278 lines); `security::secrets` (352 lines), `security::command_policy` (91 lines), `security::file_tokens` (82 lines), `security::artifact_sandbox` (314 lines), `security::redaction` (46 lines); `storage::sqlite` (475 lines), `storage::migrations` (411 lines, 4 applied migrations), `storage::fts` (169 lines), `storage::retention` (142 lines), `storage::artifacts` (294 lines); `telemetry::audit_log` (64 lines), `telemetry::release_evidence` (394 lines)
- `providers::capabilities` remains a 1-line stub (capability detection is currently implicit in `routing`)
- `storage::backup` remains a 7-line stub
- Depends on: OS APIs, SQLite (`rusqlite`), HTTP transport, system keychain (future)
- Used by: IPC command layer

**Bootstrap Layer:**
- Purpose: Wire Tauri builder, register managed state, apply migrations, register commands
- Location: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Contains: `tauri::Builder` setup, `SqlitePool::open()`, `AppState::default()`, the `tauri::generate_handler![...]` list (15 commands)
- Rule: Must stay thin; all real behavior lives in named modules

**Verification Binaries:**
- `src-tauri/src/bin/verify-command-inventory.rs` — runs `ipc::inventory::verify_inventory()` standalone
- `src-tauri/src/bin/collect-release-evidence.rs` — runs `telemetry::release_evidence::collect_release_evidence`, which calls the same inventory check before bundling evidence

## Data Flow

### Surface Preference (implemented)

1. Layout mounts → `surfaceStore.hydrate()` called in `src/routes/+layout.svelte`
2. `invoke('get_active_surface')` crosses IPC boundary
3. `ipc::app_shell::get_active_surface` calls `command_policy::policy_check`
4. Acquires `AppState.shell` mutex; if `hydrated == false`, calls `ShellPreferenceStore::load_active_surface()`
5. `ShellPreferenceStore` issues typed SQL via `SqlitePool::with_conn()` against the `shell_preferences` table
6. Returns `Surface` enum (serialized as snake_case JSON string) to renderer
7. `surfaceStore.surface` reactive rune updates; `SurfaceRail` re-renders

Surface switch (user action):
1. `surfaceStore.setSurface(next)` applies optimistic update to `$state`
2. `invoke('set_active_surface', { surface: next })` → `ipc::app_shell::set_active_surface`
3. Backend persists to SQLite first (crash-safe ordering), then updates `AppState.shell` in-memory
4. On failure: store rolls back optimistic update and sets `error` state for `StatusRegion`

### Chat Message (implemented)

1. Renderer calls `invoke('chat_send', { ... })` via the `chat` store
2. `ipc::chat::chat_send` calls `command_policy::policy_check`, validates input — no `api_key` parameter; credentials come from backend state only
3. Builds `providers::routing::RoutableMessage` values from its own `ChatMessage` type at the IPC boundary, then calls `providers::routing::build_provider_messages` (providers must never import `ipc` types directly)
4. `providers::openrouter` sends the request over the SSE transport (`providers::sse`)
5. `storage::sqlite` persists the conversation and message rows; title generation and Done/Cancel terminal writes happen here
6. `telemetry::audit_log` records a redacted trace entry
7. `chat_cancel` cancels an in-flight request via `AppState.active_requests`

### History and Search (implemented)

1. Renderer lists/searches/loads conversations via `history_list`, `history_search`, `history_get`
2. Each command calls `command_policy::policy_check`, then delegates to a typed store: `ConversationStore`, `MessageStore`, or `FtsStore` (FTS5 `MATCH` queries with `<b>`-highlighted snippets)
3. `history_delete` delegates to `RetentionStore::delete_conversation`, which runs the WAL checkpoint after a hard delete
4. No raw SQL crosses the IPC boundary — all access goes through the typed stores in `storage::sqlite`

### File Intake (implemented)

1. Renderer requests file access via `invoke('files_open_dialog')`
2. `security::file_tokens` mints an opaque token; raw path stays backend-owned in `AppState.file_tokens`
3. Renderer uses `files_read_token` for subsequent reads; it never holds the raw path directly
4. Generated artifacts are isolated by `security::artifact_sandbox` and exposed only via `artifact_get` / `artifact_dismiss`

### Provider Credentials (implemented)

1. Renderer sets/reads/clears a provider key via `privacy_set_provider_key`, `privacy_get_credential_status`, `privacy_clear_provider_key`
2. Each command is window-policy gated and delegates to `security::secrets`
3. Raw API keys never appear in an IPC response — only credential *status* crosses the boundary

## Tauri Command Surface

Commands registered in `src-tauri/src/main.rs` via `tauri::generate_handler![]` (15 total):

| Command | Module | Status | Notes |
|---------|--------|--------|-------|
| `get_active_surface` | `ipc::app_shell` | Implemented | Returns `Surface` enum; window-label enforced |
| `set_active_surface` | `ipc::app_shell` | Implemented | Persists to SQLite before updating in-memory |
| `chat_send` | `ipc::chat` | Implemented | No `api_key` parameter — credentials come from state only |
| `chat_cancel` | `ipc::chat` | Implemented | Cancels an in-flight stream |
| `artifact_get` | `ipc::artifacts` | Implemented | Reads a sandboxed artifact |
| `artifact_dismiss` | `ipc::artifacts` | Implemented | Dismisses a sandboxed artifact |
| `history_list` | `ipc::history` | Implemented | Lists conversations, most-recently-updated first |
| `history_get` | `ipc::history` | Implemented | Full conversation + message list |
| `history_delete` | `ipc::history` | Implemented | Hard delete; idempotent |
| `history_search` | `ipc::history` | Implemented | FTS5 search with highlighted snippets |
| `privacy_set_provider_key` | `ipc::privacy` | Implemented | Stores a provider credential |
| `privacy_get_credential_status` | `ipc::privacy` | Implemented | Returns credential presence, never the raw key |
| `privacy_clear_provider_key` | `ipc::privacy` | Implemented | Removes a stored credential |
| `files_open_dialog` | `ipc::files` | Implemented | Opens a native file picker, returns an opaque token |
| `files_read_token` | `ipc::files` | Implemented | Reads file content via a previously minted token |

`ipc::providers` (1-line stub) and `ipc::inventory` (726 lines, real implementation) are **not** registered in `generate_handler![]` — `inventory`'s checks run via the `verify-command-inventory` binary and release evidence collection, not as a frontend-callable command. `providers.rs` has no tracked requirement pointing at it; see `CONCERNS.md` for disposition.

**Command registration invariant:** Every command must appear in:
1. `tauri::generate_handler![...]` in `src-tauri/src/main.rs`
2. A `src-tauri/capabilities/*.json` capability grant
3. `security/command-inventory.toml` (reviewed inventory — present and reconciled)
4. `security/release-capabilities.toml`
5. Permission files under `src-tauri/permissions/`
6. `security::command_policy`'s static table

All six are reconciled by `ipc::inventory::verify_inventory()`.

## Privacy and Security Boundaries

**What stays backend-owned (never crosses to renderer):**
- Provider API keys and credentials (`security::secrets`)
- Raw file system paths (`security::file_tokens` — opaque token pattern)
- Prompt content and conversation payloads in logs or telemetry
- Raw SQL and schema details
- Provider routing decisions and model selection metadata
- `AppState` internals beyond the typed IPC response value

**Renderer enforcement model:**
- Every command validates the caller window label and command name through `security::command_policy::policy_check` (the prior per-module `assert_main_window` duplication has been removed)
- Tauri capabilities files (`src-tauri/capabilities/`) are defense-in-depth, not sole enforcement
- `withGlobalTauri: false` in `tauri.conf.json` — frontend must import specific Tauri APIs explicitly
- IPC errors serialized as `{ code: "SCREAMING_SNAKE_CASE", message: string }` — no raw Rust panics exposed
- Frontend normalizes IPC rejections via a `normalizeIpcError()` helper duplicated in each store (`chat.ts`, `surface.ts`, `history.ts`, `artifacts.ts`, `settings.ts`) — see `CONCERNS.md` for the consolidation item

**Redaction rule:** Any data path touching prompt content, secrets, raw file paths, or credentials must pass through `security::redaction` before appearing in logs, telemetry, or IPC responses.

## Architectural Constraints

- **Threading:** Single Tauri async runtime; `AppState` fields guarded by `Mutex<T>`. Lock ordering: shell lock acquired before sqlite lock (enforced in `get_active_surface`). All callers must maintain this ordering.
- **Global state:** `AppState`, `SqlitePool`, and the typed domain stores are registered as Tauri managed state via `app.manage()`. No other module-level singletons.
- **Circular imports:** None. Dependency direction: `ipc` depends on `{providers, security, storage, telemetry, app_state}`. Backend modules must not import from `ipc`; IPC-to-provider type conversions happen at the IPC boundary (e.g. `ChatMessage` → `RoutableMessage` inside `ipc::chat::chat_send`).
- **Migration ordering:** `MIGRATIONS` slice in `src-tauri/src/storage/migrations.rs` is append-only and strictly ascending by `id` (4 migrations applied: `0001`–`0004`). Never reorder or modify entries that have been applied. Migrations remain `&'static str` constants in Rust; `src-tauri/migrations/` holds only `.gitkeep`.
- **Surface enum sync:** `Surface` enum in `src-tauri/src/app_state.rs` and `type Surface` in `src/lib/stores/surface.ts` must remain in sync. Adding a new surface requires both a code change and a migration.

## Anti-Patterns

### Renderer writing to browser storage for app state

**What happens:** Using `localStorage` or `sessionStorage` to persist shell preferences or surface state.
**Why it's wrong:** Creates a split-brain between backend-owned SQLite and browser storage; breaks privacy boundary; untestable from Rust.
**Do this instead:** All app state persistence goes through `invoke(...)` → the matching `ipc::` module → its typed store. See `src/lib/stores/surface.ts`.

### IPC handler containing provider-specific logic

**What happens:** Placing OpenRouter request construction or SSE parsing inside an `ipc/` module.
**Why it's wrong:** Violates the single-concern rule; makes the command boundary untestable without a live provider.
**Do this instead:** IPC handlers call `providers::routing`, which delegates to provider adapters (`providers::openrouter`, `providers::sse`).

### Raw SQL issued from renderer or IPC layer

**What happens:** Accepting SQL strings from the frontend or constructing ad-hoc queries in `ipc/` handlers.
**Why it's wrong:** Bypasses retention policy, exposes schema, creates injection surface.
**Do this instead:** All persistence goes through typed domain stores (`ConversationStore`, `MessageStore`, `ShellPreferenceStore`, `RetentionStore`, `FtsStore` in `src-tauri/src/storage/sqlite.rs`). IPC handlers call store methods, never `with_conn` directly.

### Duplicating IPC error-normalization logic per store

**What happens:** Redefining the same `normalizeIpcError`-style function in multiple frontend store modules — currently duplicated across `chat.ts`, `surface.ts`, `history.ts`, `artifacts.ts`, and `settings.ts`.
**Why it's wrong:** The error shape is a backend-wide contract (`{ code, message }`); duplicated normalizers drift independently and obscure the single source of truth.
**Do this instead:** Extract `normalizeIpcError` into one shared module and have every store import it.

## Error Handling

**Strategy:** Typed error enums per IPC domain, serialized as structured objects.

**Patterns:**
- IPC errors use `thiserror::Error` + `serde::Serialize` with `#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]` — see `ShellError` in `src-tauri/src/ipc/app_shell.rs`, `ChatError` in `src-tauri/src/ipc/chat.rs`, `HistoryError` in `src-tauri/src/ipc/history.rs`
- Each domain error implements `From<security::command_policy::PolicyError>`, mapping both `UnauthorizedWindow` and `UnknownCommand` onto that domain's own `UnauthorizedWindow` variant
- Frontend normalizes IPC rejections via `normalizeIpcError()` in each store (not yet consolidated — see Anti-Patterns)
- Optimistic updates in stores roll back on IPC failure; error state surfaced to `StatusRegion` for accessible announcement
- Storage errors from `rusqlite` are mapped to domain error variants before crossing IPC

## Cross-Cutting Concerns

**Logging:** `console.warn` in renderer for non-fatal IPC failures; `telemetry::audit_log` for backend traces — redaction required before persistence.
**Validation:** Input validated at IPC boundary before any backend module is invoked.
**Authentication:** Provider credentials held exclusively in `security::secrets`; never in IPC responses, logs, or frontend state — only credential *status* crosses the boundary.

---

*Architecture analysis: 2026-06-18*
