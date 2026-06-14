<!-- refreshed: 2026-06-13 -->
# Architecture

**Analysis Date:** 2026-06-13

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
│  - Validates caller window label (backend-side enforcement)      │
│  - Validates input at IPC boundary                               │
│  - Returns structured, typed results                             │
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
| IPC surface | Typed Tauri commands callable from renderer; window-label enforcement | `src-tauri/src/ipc/mod.rs`, `src-tauri/src/ipc/app_shell.rs` |
| AppState | In-memory runtime state (ShellState, active surface); `Send + Sync` singleton | `src-tauri/src/app_state.rs` |
| providers | Capability detection, provider routing, OpenRouter transport, SSE streaming | `src-tauri/src/providers/` |
| security | Secrets store, file-access tokens, redaction, command policy, artifact sandbox | `src-tauri/src/security/` |
| storage | SQLite pool (WAL), typed domain stores, migration runner, FTS, retention, backup | `src-tauri/src/storage/` |
| telemetry | Audit log, release evidence capture (redaction-gated) | `src-tauri/src/telemetry/` |
| Svelte renderer | UI surfaces (Chat, History, Settings, Artifacts), accessibility, surface navigation | `src/lib/components/`, `src/routes/` |
| surface store | Frontend singleton bridging IPC and Svelte 5 reactive state | `src/lib/stores/surface.ts` |

## Pattern Overview

**Overall:** Docs-led desktop client scaffold with strict process-boundary privacy enforcement.

**Key Characteristics:**
- Backend owns all persistence, provider credentials, file authority, and routing decisions
- Renderer is treated as a potentially hostile surface — no secrets, raw paths, or SQL cross the IPC boundary
- IPC commands are the only API surface: validated at entry, returning typed structured results
- Tauri `AppState` holds in-memory session state; `ShellPreferenceStore` bridges it to SQLite
- Svelte 5 runes (`$state`, `$derived`) for reactive frontend state; no `localStorage` or `sessionStorage`

## Layers

**Renderer Layer:**
- Purpose: Render UI surfaces and mediate user intent into IPC calls
- Location: `src/`
- Contains: SvelteKit routes, Svelte 5 components, typed stores, accessibility helpers
- Depends on: `@tauri-apps/api/core` for `invoke`; no direct OS or storage access
- Used by: end user via the Tauri WebView window

**IPC Command Layer:**
- Purpose: Validate and dispatch renderer requests to backend modules
- Location: `src-tauri/src/ipc/`
- Contains: One submodule per domain (app_shell, chat, files, history, inventory, privacy, providers)
- Depends on: `app_state`, `security`, `storage`, `providers`, `telemetry`
- Used by: renderer only, via Tauri `invoke`

**Business Logic Modules:**
- Purpose: Implement backend-owned concerns; each module owns exactly one concern
- Location: `src-tauri/src/{providers,security,storage,telemetry}/`
- Contains: Domain types, store implementations, routing logic, redaction, migration runner
- Depends on: OS APIs, SQLite (`rusqlite`), HTTP transport, system keychain (future)
- Used by: IPC command layer

**Bootstrap Layer:**
- Purpose: Wire Tauri builder, register managed state, apply migrations, register commands
- Location: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Contains: `tauri::Builder` setup, `SqlitePool::open()`, `AppState::default()`
- Rule: Must stay thin; all real behavior lives in named modules

## Data Flow

### Surface Preference (implemented)

1. Layout mounts → `surfaceStore.hydrate()` called in `src/routes/+layout.svelte`
2. `invoke('get_active_surface')` crosses IPC boundary
3. `ipc::app_shell::get_active_surface` validates window label (`main`)
4. Acquires `AppState.shell` mutex; if `hydrated == false`, calls `ShellPreferenceStore::load_active_surface()`
5. `ShellPreferenceStore` issues typed SQL via `SqlitePool::with_conn()` against `shell_preferences` table
6. Returns `Surface` enum (serialized as snake_case JSON string) to renderer
7. `surfaceStore.surface` reactive rune updates; `SurfaceRail` re-renders

Surface switch (user action):
1. `surfaceStore.setSurface(next)` applies optimistic update to `$state`
2. `invoke('set_active_surface', { surface: next })` → `ipc::app_shell::set_active_surface`
3. Backend persists to SQLite first (crash-safe ordering), then updates `AppState.shell` in-memory
4. On failure: store rolls back optimistic update and sets `error` state for `StatusRegion`

### Chat Message (scaffolded, not yet implemented)

1. Renderer calls `invoke('chat_send', { ... })` via `ipc::chat`
2. IPC handler validates input, invokes `security::command_policy` check
3. Forwards to `providers::routing` → selects provider via capability detection
4. Provider adapter (`providers::openrouter`) sends request over SSE transport (`providers::sse`)
5. Streaming chunks returned to renderer via Tauri events (pattern TBD)
6. `storage::sqlite` persists conversation record
7. `telemetry::audit_log` records redacted trace entry

### File Intake (scaffolded, not yet implemented)

1. Renderer requests file access → `invoke('files_request_token', { ... })`
2. `security::file_tokens` mints an opaque token; raw path stays backend-owned
3. Renderer uses token for subsequent file operations; never holds path directly
4. `security::artifact_sandbox` isolates any generated artifacts

## Tauri Command Surface

Commands registered in `src-tauri/src/main.rs` via `tauri::generate_handler![]`:

| Command | Module | Status | Notes |
|---------|--------|--------|-------|
| `get_active_surface` | `ipc::app_shell` | Implemented | Returns `Surface` enum; window-label enforced |
| `set_active_surface` | `ipc::app_shell` | Implemented | Persists to SQLite before updating in-memory |
| `chat_*` | `ipc::chat` | Scaffolded | Placeholder only; not yet registered |
| `files_*` | `ipc::files` | Scaffolded | Placeholder only; not yet registered |
| `history_*` | `ipc::history` | Scaffolded | Placeholder only; not yet registered |
| `providers_*` | `ipc::providers` | Scaffolded | Placeholder only; not yet registered |
| `privacy_*` | `ipc::privacy` | Scaffolded | Placeholder only; not yet registered |
| `inventory_*` | `ipc::inventory` | Scaffolded | Placeholder only; not yet registered |

**Command registration invariant:** Every command must appear in:
1. `tauri::generate_handler![...]` in `src-tauri/src/main.rs`
2. A `src-tauri/capabilities/*.json` capability grant
3. `security/command-inventory.toml` (reviewed inventory — file not yet present)

## Privacy and Security Boundaries

**What stays backend-owned (never crosses to renderer):**
- Provider API keys and credentials (`security::secrets`)
- Raw file system paths (`security::file_tokens` — opaque token pattern)
- Prompt content and conversation payloads in logs or telemetry
- Raw SQL and schema details
- Provider routing decisions and model selection metadata
- `AppState` internals beyond the typed IPC response value

**Renderer enforcement model:**
- Window label checked on every shell command (`assert_main_window` in `src-tauri/src/ipc/app_shell.rs`)
- Tauri capabilities files (`src-tauri/capabilities/`) are defense-in-depth, not sole enforcement
- `withGlobalTauri: false` in `tauri.conf.json` — frontend must import specific Tauri APIs explicitly
- IPC errors serialized as `{ code: "SCREAMING_SNAKE_CASE", message: string }` — no raw Rust panics exposed
- Frontend normalizes IPC rejections via `normalizeIpcError()` in `src/lib/stores/surface.ts`

**Redaction rule:** Any data path touching prompt content, secrets, raw file paths, or credentials must pass through `security::redaction` before appearing in logs, telemetry, or IPC responses.

## Architectural Constraints

- **Threading:** Single Tauri async runtime; `AppState` fields guarded by `Mutex<T>`. Lock ordering: shell lock acquired before sqlite lock (enforced in `get_active_surface`). All callers must maintain this ordering.
- **Global state:** `AppState`, `SqlitePool`, and `ShellPreferenceStore` registered as Tauri managed state via `app.manage()`. No other module-level singletons.
- **Circular imports:** None currently. Dependency direction: `ipc` depends on `{providers, security, storage, telemetry, app_state}`. Backend modules must not import from `ipc`.
- **Migration ordering:** `MIGRATIONS` slice in `src-tauri/src/storage/migrations.rs` is append-only and strictly ascending by `id`. Never reorder or modify entries that have been applied.
- **Surface enum sync:** `Surface` enum in `src-tauri/src/app_state.rs` and `type Surface` in `src/lib/stores/surface.ts` must remain in sync. Adding a new surface requires both a code change and a migration.

## Anti-Patterns

### Renderer writing to browser storage for app state

**What happens:** Using `localStorage` or `sessionStorage` to persist shell preferences or surface state.
**Why it's wrong:** Creates a split-brain between backend-owned SQLite and browser storage; breaks privacy boundary; untestable from Rust.
**Do this instead:** All app state persistence goes through `invoke('set_active_surface', ...)` → `ipc::app_shell` → `ShellPreferenceStore`. See `src/lib/stores/surface.ts`.

### IPC handler containing provider-specific logic

**What happens:** Placing OpenRouter request construction or SSE parsing inside an `ipc/` module.
**Why it's wrong:** Violates the single-concern rule; makes the command boundary untestable without a live provider.
**Do this instead:** IPC handlers call `providers::routing` which delegates to provider adapters (`providers::openrouter`, `providers::sse`).

### Raw SQL issued from renderer or IPC layer

**What happens:** Accepting SQL strings from the frontend or constructing ad-hoc queries in `ipc/` handlers.
**Why it's wrong:** Bypasses retention policy, exposes schema, creates injection surface.
**Do this instead:** All persistence goes through typed domain stores (e.g. `ShellPreferenceStore` in `src-tauri/src/storage/sqlite.rs`). IPC handlers call store methods, never `with_conn` directly.

## Error Handling

**Strategy:** Typed error enums per IPC domain, serialized as structured objects.

**Patterns:**
- IPC errors use `thiserror::Error` + `serde::Serialize` with `#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]` — see `ShellError` in `src-tauri/src/ipc/app_shell.rs`
- Frontend normalizes IPC rejections via `normalizeIpcError()` in `src/lib/stores/surface.ts`
- Optimistic updates in stores roll back on IPC failure; error state surfaced to `StatusRegion` for accessible announcement
- Storage errors from `rusqlite` are mapped to domain error variants before crossing IPC

## Cross-Cutting Concerns

**Logging:** `console.warn` in renderer for non-fatal IPC failures; `telemetry::audit_log` for backend traces — redaction required before persistence.
**Validation:** Input validated at IPC boundary before any backend module is invoked.
**Authentication:** Provider credentials held exclusively in `security::secrets` (scaffolded); never in IPC responses, logs, or frontend state.

---

*Architecture analysis: 2026-06-13*
