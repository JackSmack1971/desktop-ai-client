# Architecture

## System shape

This is a Tauri desktop application: a Svelte 5 renderer (single `main` window)
talking to a Rust backend exclusively through typed Tauri IPC commands. There
is no agent loop, planner, or memory-promotion pipeline — the backend's job is
to own privacy- and security-sensitive concerns (provider credentials, file
paths, SQLite storage, command policy, telemetry) and expose only typed,
validated commands to the renderer.

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

## Component responsibilities

| Component | Responsibility | Key files |
|-----------|----------------|-----------|
| IPC surface | Typed Tauri commands callable from the renderer; window-label and command-name enforcement | `src-tauri/src/ipc/*.rs` |
| Command policy | Single authority for "is this command callable from this window" | `src-tauri/src/security/command_policy.rs` |
| AppState | In-memory runtime state (`ShellState`, active surface); `Send + Sync` singleton | `src-tauri/src/app_state.rs` |
| providers | Capability detection, provider routing, OpenRouter transport, SSE streaming | `src-tauri/src/providers/` |
| security | Secrets store, file-access tokens, redaction, command policy, artifact sandbox | `src-tauri/src/security/` |
| storage | SQLite pool (WAL), typed domain stores, migration runner, FTS, retention, backup | `src-tauri/src/storage/` |
| telemetry | Audit log, release evidence capture (redaction-gated) | `src-tauri/src/telemetry/` |
| Svelte renderer | UI surfaces (chat, history, surfaces shell), accessibility, surface navigation | `src/lib/components/`, `src/routes/` |
| Frontend stores | Typed stores bridging IPC and Svelte 5 reactive state | `src/lib/stores/*.ts` |

## Layers

**Renderer layer**
- Purpose: render UI surfaces and turn user intent into IPC calls
- Location: `src/`
- Contains: SvelteKit routes, Svelte 5 components, typed stores (`chat.ts`, `history.ts`, `surface.ts`, `artifacts.ts`, `settings.ts`), accessibility helpers
- Depends on: `@tauri-apps/api/core` for `invoke`; no direct OS or storage access
- Used by: the end user via the Tauri WebView window

**IPC command layer**
- Purpose: validate and dispatch renderer requests to backend modules
- Location: `src-tauri/src/ipc/`
- Contains: one submodule per domain — `app_shell`, `chat`, `history`, `artifacts`, `privacy`, `files`, `inventory`; `providers` exists as an unregistered placeholder
- Depends on: `app_state`, `security`, `storage`, `providers`, `telemetry`
- Used by: the renderer only, via Tauri `invoke`
- Rule: backend modules under `providers/security/storage/telemetry` must never import from `ipc` — conversions between IPC wire types and provider-owned types happen at the IPC boundary (e.g. `ipc::chat::chat_send` builds `providers::routing::RoutableMessage` from its own `ChatMessage`, instead of `providers::routing` depending on `ipc` types)

**Business logic modules**
- Purpose: implement backend-owned concerns; each module owns exactly one concern
- Location: `src-tauri/src/{providers,security,storage,telemetry}/`
- Contains: domain types, store implementations, routing logic, redaction, migration runner
- Depends on: OS APIs, SQLite (`rusqlite`), HTTP transport, system keychain (future)
- Used by: the IPC command layer

**Bootstrap layer**
- Purpose: wire the Tauri builder, register managed state, apply migrations, register commands
- Location: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Contains: `tauri::Builder` setup, `SqlitePool::open()`, `AppState::default()`, the `tauri::generate_handler![...]` command list
- Rule: must stay thin; all real behavior lives in named modules

## Command policy and the inventory invariant

`security::command_policy::policy_check(command, window_label)` is the single
authority every IPC command calls before doing any work. It rejects both
unknown command names and calls from a window label that isn't allowed to
invoke that command.

The set of commands known to the policy table must agree with five other
sources of truth, checked by `ipc::inventory::verify_inventory()`
(`cargo run --bin verify-command-inventory`):

1. `security/command-inventory.toml` — the reviewed inventory
2. Commands registered in `tauri::generate_handler![...]` in `main.rs`
3. Permission files under `src-tauri/permissions/`
4. Capability grants in `src-tauri/capabilities/*.json`
5. `security/release-capabilities.toml`
6. `security::command_policy::command_names()` — the policy table itself

`telemetry::release_evidence::collect_release_evidence` calls the same
`verify_inventory()` check, so a drift between any of these sources fails
release evidence collection, not just the standalone binary.

## Data flow

### Surface preference

1. Layout mounts → `surfaceStore.hydrate()` called in `src/routes/+layout.svelte`
2. `invoke('get_active_surface')` crosses the IPC boundary
3. `ipc::app_shell::get_active_surface` calls `command_policy::policy_check`
4. Acquires the `AppState.shell` mutex; if `hydrated == false`, calls `ShellPreferenceStore::load_active_surface()` (lock ordering: shell lock before sqlite lock)
5. `ShellPreferenceStore` issues typed SQL via `SqlitePool::with_conn()` against the `shell_preferences` table
6. Returns the `Surface` enum (serialized as a snake_case JSON string) to the renderer
7. `surfaceStore.surface` reactive rune updates; `SurfaceRail` re-renders

Surface switch (user action):
1. `surfaceStore.setSurface(next)` applies an optimistic update to `$state`
2. `invoke('set_active_surface', { surface: next })` → `ipc::app_shell::set_active_surface`
3. Backend persists to SQLite first (crash-safe ordering), then updates `AppState.shell` in-memory
4. On failure: the store rolls back the optimistic update and sets `error` state for `StatusRegion`

### Chat message

1. Renderer calls `invoke('chat_send', { ... })` via the `chat` store
2. `ipc::chat::chat_send` calls `command_policy::policy_check`, then validates input
3. Builds `providers::routing::RoutableMessage` values from its own `ChatMessage` type (the IPC ↔ provider type conversion boundary) and calls `providers::routing::build_provider_messages`
4. `providers::routing` selects a provider via capability detection (`providers::capabilities`)
5. The provider adapter (`providers::openrouter`) sends the request over the SSE transport (`providers::sse`)
6. `storage::sqlite` persists the conversation and message rows
7. `telemetry::audit_log` records a redacted trace entry
8. `chat_cancel` cancels an in-flight request; both commands are window-policy gated

### History and search

1. Renderer lists/searches/loads conversations via `history_list`, `history_search`, `history_get`
2. Each command calls `command_policy::policy_check`, then delegates to a typed store: `ConversationStore`, `MessageStore`, or `FtsStore` (FTS5 `MATCH` queries with snippet highlighting)
3. `history_delete` delegates to `RetentionStore::delete_conversation`, which runs the WAL checkpoint after a hard delete
4. No raw SQL crosses the IPC boundary — all access goes through the typed stores in `storage::sqlite`

### File intake

1. Renderer requests file access via `files_open_dialog`
2. `security::file_tokens` mints an opaque token; the raw path stays backend-owned
3. The renderer uses `files_read_token` for subsequent reads; it never holds the raw path directly
4. Generated artifacts are isolated by `security::artifact_sandbox` and exposed only via `artifact_get` / `artifact_dismiss`

### Provider credentials

1. Renderer sets/reads/clears a provider key via `privacy_set_provider_key`, `privacy_get_credential_status`, `privacy_clear_provider_key`
2. Each command is window-policy gated and delegates to `security::secrets`
3. Raw API keys never appear in an IPC response — only credential *status* crosses the boundary

## Tauri command surface

Commands registered in `src-tauri/src/main.rs` via `tauri::generate_handler![]`:

| Command | Module | Notes |
|---------|--------|-------|
| `get_active_surface` | `ipc::app_shell` | Returns the `Surface` enum; window-label enforced |
| `set_active_surface` | `ipc::app_shell` | Persists to SQLite before updating in-memory state |
| `chat_send` | `ipc::chat` | No `api_key` parameter — credentials come from backend state only |
| `chat_cancel` | `ipc::chat` | Cancels an in-flight stream |
| `history_list` | `ipc::history` | Lists conversations, most-recently-updated first |
| `history_get` | `ipc::history` | Full conversation + message list |
| `history_delete` | `ipc::history` | Hard delete; idempotent |
| `history_search` | `ipc::history` | FTS5 search with highlighted snippets |
| `artifact_get` | `ipc::artifacts` | Reads a sandboxed artifact |
| `artifact_dismiss` | `ipc::artifacts` | Dismisses a sandboxed artifact |
| `privacy_set_provider_key` | `ipc::privacy` | Stores a provider credential |
| `privacy_get_credential_status` | `ipc::privacy` | Returns credential presence, never the raw key |
| `privacy_clear_provider_key` | `ipc::privacy` | Removes a stored credential |
| `files_open_dialog` | `ipc::files` | Opens a native file picker, returns an opaque token |
| `files_read_token` | `ipc::files` | Reads file content via a previously minted token |

`ipc::providers` and `ipc::inventory` exist as modules but are not registered
in `generate_handler![]` — `inventory`'s checks run via the
`verify-command-inventory` binary and release evidence collection, not as a
frontend-callable command.

**Command registration invariant:** every frontend-callable command must
appear in `tauri::generate_handler![...]`, a `src-tauri/capabilities/*.json`
grant, `security/command-inventory.toml`, and
`security::command_policy`'s table — see "Command policy and the inventory
invariant" above.

## Privacy and security boundaries

**What stays backend-owned (never crosses to the renderer):**
- Provider API keys and credentials (`security::secrets`)
- Raw file system paths (`security::file_tokens` — opaque token pattern)
- Prompt content and conversation payloads in logs or telemetry
- Raw SQL and schema details
- Provider routing decisions and model selection metadata
- `AppState` internals beyond the typed IPC response value

**Renderer enforcement model:**
- Every command validates the caller window label and command name through `security::command_policy::policy_check`
- Tauri capability files (`src-tauri/capabilities/`) are defense-in-depth, not the sole enforcement layer
- `app.withGlobalTauri: false` in `tauri.conf.json` — frontend code must import specific Tauri APIs explicitly
- IPC errors serialize as `{ code: "SCREAMING_SNAKE_CASE", message: string }` — no raw Rust panics exposed
- The frontend normalizes IPC rejections via a `normalizeIpcError()` helper in each store module

**Redaction rule:** any data path touching prompt content, secrets, raw file paths, or credentials must pass through `security::redaction` before appearing in logs, telemetry, or IPC responses.

## Architectural constraints

- **Threading:** single Tauri async runtime; `AppState` fields guarded by `Mutex<T>`. Lock ordering: shell lock acquired before the sqlite lock (enforced in `get_active_surface`). All callers must maintain this ordering.
- **Global state:** `AppState` and the SQLite pool/stores are registered as Tauri managed state via `app.manage()`. No other module-level singletons.
- **Layering:** `ipc` depends on `{providers, security, storage, telemetry, app_state}`. Backend modules must never import from `ipc`; type conversions happen at the IPC boundary.
- **Migration ordering:** the migrations slice in `src-tauri/src/storage/migrations.rs` is append-only and strictly ascending by id. Never reorder or modify entries that have already been applied.
- **Surface enum sync:** the `Surface` enum in `src-tauri/src/app_state.rs` and the corresponding type in `src/lib/stores/surface.ts` must remain in sync. Adding a new surface requires both a code change and a migration.

## Anti-patterns

### Renderer writing to browser storage for app state

**What happens:** using `localStorage` or `sessionStorage` to persist shell preferences, conversation state, or surface state.
**Why it's wrong:** creates a split-brain between backend-owned SQLite and browser storage; breaks the privacy boundary; untestable from Rust.
**Do this instead:** all app state persistence goes through a typed `invoke(...)` call into the matching `ipc::` module and its backing store. See `src/lib/stores/surface.ts`.

### IPC handler containing provider-specific logic

**What happens:** placing OpenRouter request construction or SSE parsing inside an `ipc/` module.
**Why it's wrong:** violates the single-concern rule; makes the command boundary untestable without a live provider.
**Do this instead:** IPC handlers call `providers::routing`, which delegates to provider adapters (`providers::openrouter`, `providers::sse`).

### Raw SQL issued from the renderer or IPC layer

**What happens:** accepting SQL strings from the frontend or constructing ad-hoc queries in `ipc/` handlers.
**Why it's wrong:** bypasses retention policy, exposes schema, creates an injection surface.
**Do this instead:** all persistence goes through typed domain stores in `storage::sqlite` (e.g. `ConversationStore`, `MessageStore`, `ShellPreferenceStore`, `RetentionStore`, `FtsStore`). IPC handlers call store methods, never raw SQL.

### Duplicating IPC error-normalization logic per store

**What happens:** redefining the same `normalizeIpcError`-style function in multiple frontend store modules (currently duplicated across `chat.ts`, `surface.ts`, `history.ts`, `artifacts.ts`, and `settings.ts`).
**Why it's wrong:** the error shape is a backend-wide contract (`{ code, message }`); duplicated normalizers drift independently and obscure the single source of truth.
**Do this instead:** extract `normalizeIpcError` into one shared module and have every store import it.

## Error handling

**Strategy:** typed error enums per IPC domain, serialized as structured objects.

**Patterns:**
- IPC errors use `thiserror::Error` + `serde::Serialize` with `#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]` — see `ShellError` in `src-tauri/src/ipc/app_shell.rs`, `ChatError` in `src-tauri/src/ipc/chat.rs`, `HistoryError` in `src-tauri/src/ipc/history.rs`
- Each domain error implements `From<security::command_policy::PolicyError>` so a policy rejection surfaces as that domain's own `UnauthorizedWindow` variant
- The frontend normalizes IPC rejections via a `normalizeIpcError()` helper in each store (duplicated today — see "Duplicating IPC error-normalization logic per store" above)
- Optimistic updates in stores roll back on IPC failure; error state surfaces to `StatusRegion` for accessible announcement
- Storage errors from `rusqlite` are mapped to domain error variants before crossing the IPC boundary

## Cross-cutting concerns

**Logging:** `console.warn` in the renderer for non-fatal IPC failures; `telemetry::audit_log` for backend traces — redaction required before persistence.
**Validation:** input validated at the IPC boundary before any backend module is invoked.
**Authentication:** provider credentials held exclusively in `security::secrets`; never returned in IPC responses, logs, or frontend state — only credential *status* crosses the boundary.

## Prompting boundary

Prompt design rules live in `docs/prompt-blueprint.md`. Use that file
whenever changing system prompts, developer prompts, task prompts, or
routing prompts sent to a provider.
