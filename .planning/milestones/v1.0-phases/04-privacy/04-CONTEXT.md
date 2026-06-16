# Phase 4: Privacy - Context

**Gathered:** 2026-06-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Enforce the privacy boundary for secrets, file access, and telemetry. This phase implements:
- `security::secrets` — replace Phase 2's env-var `EnvSecretStore` with `KeyringSecretStore` (OS keychain via `keyring` crate). Caller interface unchanged.
- `security::file_tokens` — session-scoped opaque token system. Backend opens native OS file picker, mints UUID tokens; frontend never holds raw paths.
- `security::redaction` — unconditional redaction of secrets, file paths, and content-bearing data before any log or telemetry write.
- `security::command_policy` — static allow-table mapping IPC commands to allowed window labels; typed `deny_unknown_fields` structs block forbidden parameters.
- `ipc::privacy` — three commands: `privacy_set_provider_key`, `privacy_get_credential_status`, `privacy_clear_provider_key`.
- `ipc::files` — minimal file dialog commands: `files_open_dialog` (native picker → token), `files_read_token`.
- `telemetry::audit_log` — append-only JSON Lines audit file; schema validation + redaction before every write.
- `SettingsSurface.svelte` — minimal credential management UI (set/clear/status). Narrow scope only.

This phase does NOT include: `security::artifact_sandbox` (Phase 5), full Settings experience (model picker, retention config), wiring attachment tokens into `chat_send` (Phase 5), persisted attachment metadata, or the Phase 6 command-inventory release verifier.

</domain>

<decisions>
## Implementation Decisions

### Secrets Backing Store

- **D-01:** Phase 4 replaces `EnvSecretStore` with `KeyringSecretStore` using the Rust `keyring` crate (macOS Keychain, Windows Credential Manager, Linux SecretService). `EnvSecretStore` remains available only behind an explicit dev/test configuration flag for CI and local development. Production must fail closed — if OS keychain is unavailable, `get_provider_key()` returns `SecretsError::NotConfigured`; the frontend sees `CredentialStatus::Missing`.
- **D-02:** Primary credential write path: `privacy_set_provider_key` IPC command from the Settings UI. The frontend sends the key exactly once; Rust stores it in the OS keychain. The frontend never receives or holds the key again. An env-var import helper may exist behind an explicit dev/test/migration flag, but production must never silently fall back to env vars.

### Command Policy

- **D-03:** `security::command_policy` is implemented in Phase 4 as a static runtime allow-table mapping IPC command names to permitted window labels. Every IPC handler in Phase 4 calls `policy_check()` before doing work. Forbidden payload fields are prevented by typed request structs with `deny_unknown_fields`. Phase 6 later replaces or augments the hardcoded table with the reviewed command-inventory verifier — design `policy_check()` callers so that Phase 6 can swap the backing table without changing call sites.

### File Access Token System

- **D-04:** File selection is Rust-owned. The backend calls `tauri-plugin-dialog` to open the native OS file picker, receives the path directly from the OS, validates it, mints an opaque UUID token, and returns only the token plus safe metadata (filename, MIME type, size) to the frontend. `chat_send` must accept attachment tokens, never raw filesystem paths.
- **D-05:** File access tokens are session-scoped and in-memory only. The token map is a `Mutex<HashMap<TokenId, PathBuf>>` field in `AppState`. On app quit all tokens are dropped. Safe attachment metadata may be persisted in a future phase; token authority and source paths are never persisted.
- **D-06:** Phase 4 implements `security::file_tokens` and minimal `ipc::files` commands (`files_open_dialog`, `files_read_token`). Wiring attachment tokens into `chat_send` is deferred to Phase 5.

### Redaction Pipeline

- **D-07:** Three categories must pass through `security::redaction` before any log or telemetry write: (1) provider API keys and credentials, (2) raw file paths, (3) all content-bearing data (prompt text, message content, attachment content). Prompt and message content is normally omitted entirely — redaction is the safety net, not the primary strategy.
- **D-08:** `telemetry::audit_log` writes to an append-only JSON Lines file in the app data logs directory (e.g., `<app_data>/logs/audit.log`). Schema validation and redaction must run before every write. A SQLite `audit_events` table is not implemented in this phase.
- **D-09:** Redaction is unconditional. No debug or dev-mode escape hatch lowers content or path redaction. Dev mode may emit additional metadata-level logs (IPC command names, timestamps, status codes) but never content, paths, or credential material.

### IPC Privacy Surface

- **D-10:** `ipc::privacy` exposes exactly three commands: `privacy_set_provider_key`, `privacy_get_credential_status`, `privacy_clear_provider_key`. No IPC command ever returns or exposes the key value. Frontend commands can only write, clear, and query status.
- **D-11:** Phase 4 includes a minimal `SettingsSurface.svelte` implementing only credential management: API key entry (write-only), credential status display, and clear. Scope is narrow and security-focused. Model selection, retention config, theme, and broader Settings UX are deferred.

### Claude's Discretion

- Keyring service name and account label format for keychain entries (e.g., `"desktop-ai-client"` / `"openrouter"`)
- Exact fields in the safe metadata returned by `files_open_dialog` (filename, size, MIME type are expected)
- JSON Lines schema for audit log entries (which fields per event type: IPC command name, timestamp, window label, status code — no payload content)
- Error enum naming for `PrivacyError` and `FilesError` variants (follow `ShellError` from `ipc/app_shell.rs`)
- In-memory token ID type (`uuid::Uuid` or a newtype wrapper)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope and Requirements
- `.planning/ROADMAP.md` — Phase 4 goal, success criteria (SEC-01, SEC-02, SEC-03)
- `.planning/REQUIREMENTS.md` — SEC-01, SEC-02, SEC-03 definitions

### Architecture and Patterns
- `.planning/codebase/ARCHITECTURE.md` — Security module responsibilities (§Component Responsibilities), IPC boundary enforcement (§Privacy and Security Boundaries), file intake data flow (§Data Flow: File Intake), command registration invariant (§Tauri Command Surface), error handling patterns (§Error Handling), threading/lock ordering (§Architectural Constraints)
- `src-tauri/src/ipc/app_shell.rs` — Canonical IPC patterns: `ShellError` typed error enum with `thiserror + serde`, `assert_main_window` window-label enforcement. New `PrivacyError` and `FilesError` must follow the same serialization shape.

### Security and Privacy Authority
- `docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md` — **Primary authority** for IPC boundary invariants, secret handling rules, file access constraints, and redaction requirements. Read before designing any Phase 4 interface.
- `docs/privacy-boundaries.md` — Stub; populate before implementation. Focus areas: secrets handling, file content visibility, telemetry redaction, local storage scope.
- `docs/threat-model.md` — Stub; populate before implementation. Focus areas: secret exposure, hostile renderer behavior, file access boundary violations.

### Phase 2 Context (upstream decisions Phase 4 must honor)
- `.planning/phases/02-routing/02-CONTEXT.md` — D-07 (Phase 4 replaces secrets backing store), D-08 (`get_provider_key()` + `get_credential_status()` caller signatures locked), D-10 (hard invariant: IPC commands must NEVER accept `api_key` parameter)

### Scaffolded Files to Implement
- `src-tauri/src/security/secrets.rs` — Phase 2 stub (fully implemented). Phase 4 replaces `SecretsState` backing with `KeyringSecretStore`; `ProviderId`, `SecretsError`, `CredentialStatus`, and the two public functions are locked.
- `src-tauri/src/security/file_tokens.rs` — Scaffold; implement `mint_token()`, `resolve_token()`, `revoke_token()` against `AppState.file_tokens` map.
- `src-tauri/src/security/redaction.rs` — Scaffold; implement redaction for the three categories (secrets, paths, content-bearing data).
- `src-tauri/src/security/command_policy.rs` — Scaffold; implement static allow-table with `policy_check()` callable by every IPC handler.
- `src-tauri/src/ipc/privacy.rs` — Scaffold; implement three commands.
- `src-tauri/src/ipc/files.rs` — Scaffold; implement `files_open_dialog` and `files_read_token`.
- `src-tauri/src/telemetry/audit_log.rs` — Scaffold; implement JSON Lines writer with redaction gate.
- `src/lib/components/surfaces/SettingsSurface.svelte` — Scaffold; implement minimal credential UI.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src-tauri/src/ipc/app_shell.rs` — `assert_main_window()` helper; `ShellError` enum with `thiserror + serde + #[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]` derive pattern. Copy for `PrivacyError`, `FilesError`.
- `src-tauri/src/security/secrets.rs` — Already fully implemented in Phase 2. `ProviderId` enum, `SecretsError`, `CredentialStatus`, `get_provider_key()`, `get_credential_status()` — callers unchanged. Phase 4 replaces the `SecretsState` struct's backing store only.
- `src-tauri/src/storage/sqlite.rs` — `SqlitePool` and `AppState` management patterns. File token map (`Mutex<HashMap<TokenId, PathBuf>>`) must be added as a new field in `src-tauri/src/app_state.rs`.
- `src/lib/stores/surface.ts` — `normalizeIpcError()` for frontend IPC error normalization. Settings UI must use the same pattern.

### Established Patterns
- **IPC error shape:** `{ code: "SCREAMING_SNAKE_CASE", message: string }` — all Phase 4 error enums must serialize to this shape.
- **Window-label enforcement:** Phase 4 replaces the per-handler `assert_main_window` call with `command_policy::policy_check()` which performs the same check via the allow-table.
- **Typed structs with `deny_unknown_fields`:** All Phase 4 IPC request structs use `#[serde(deny_unknown_fields)]` to prevent forbidden parameter smuggling.
- **Lock ordering:** shell lock before sqlite lock (established in Phase 1). File token map lock must not invert this ordering.
- **Command registration invariant:** Every new command must appear in `tauri::generate_handler![...]` (main.rs) AND a `src-tauri/capabilities/*.json` grant.

### Integration Points
- `src-tauri/src/main.rs` — Must register all Phase 4 IPC commands in `tauri::generate_handler![...]`.
- `src-tauri/src/app_state.rs` — Must extend `AppState` with `file_tokens: Mutex<HashMap<TokenId, PathBuf>>`.
- `src-tauri/capabilities/main.json` — Must add allow grants for all new Phase 4 commands.
- `src-tauri/src/ipc/chat.rs` (Phase 2/3 output) — `chat_send` signature will need an attachment token parameter in Phase 5; design file token API with this downstream use in mind.

</code_context>

<specifics>
## Specific Ideas

- Production fail-closed: if OS keychain access is unavailable (e.g., keychain locked, permission denied), `get_provider_key()` returns `SecretsError::NotConfigured` — same error as key not present. Frontend shows `CredentialStatus::Missing` and prompts user to reconfigure. No silent fallback to env var in production.
- `privacy_set_provider_key` must never log or echo the key value in any code path — not in error messages, not in debug output, not in audit log entries.
- The `command_policy` allow-table should be designed so Phase 6 can replace the hardcoded map with a loaded `security/command-inventory.toml` without changing `policy_check()` callers. Suggest a `CommandPolicy` trait or struct with a `check()` method.
- `files_open_dialog` opens the native OS picker backend-side; it does not accept a path from the frontend. If the user cancels the dialog, return a specific `FilesError::Cancelled` variant (not a generic error), so the frontend can distinguish user-cancelled from system failures.
- Safe metadata returned by `files_open_dialog`: filename (not full path), size in bytes, MIME type. These are safe to cross the IPC boundary because they carry no path authority.

</specifics>

<deferred>
## Deferred Ideas

- **`chat_send` attachment wiring** — Accept attachment tokens in `chat_send`. Deferred to Phase 5 when the full attach flow is needed alongside artifact generation.
- **Persisted attachment metadata** — Safe metadata (filename, type, size) may be stored in SQLite for conversation history display. Deferred to Phase 5; token authority and source paths must never be persisted.
- **`security::artifact_sandbox`** — Phase 5 (Artifacts surface). Isolates generated artifacts from the host UI.
- **Full Settings experience** — Model selection, retention config, theme, notification settings, etc. Deferred to a future settings phase.
- **SQLite `audit_events` table** — A queryable in-app audit log. Deferred; JSON Lines file is sufficient for Phase 4.
- **`privacy_import_from_env` helper** — Dev/test utility that reads the env-var key and writes it to the keychain. If implemented, make it a CLI flag or test helper, not an IPC command.
- **Phase 6 command-inventory verifier** — `command_policy` allow-table is a runtime enforcement layer; the full deny-by-inventory release gate (cross-checks against `security/command-inventory.toml`) is Phase 6 scope.

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 04-privacy*
*Context gathered: 2026-06-15*
