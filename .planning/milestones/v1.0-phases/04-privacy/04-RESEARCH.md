# Phase 4: Privacy - Research

**Researched:** 2026-06-15
**Domain:** Rust security modules — OS keychain, file access tokens, redaction, command policy, audit log, Svelte credentials UI
**Confidence:** HIGH

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Phase 4 replaces `EnvSecretStore` with `KeyringSecretStore` using the Rust `keyring` crate (macOS Keychain, Windows Credential Manager, Linux SecretService). `EnvSecretStore` remains available only behind an explicit dev/test configuration flag. Production must fail closed — if OS keychain is unavailable, `get_provider_key()` returns `SecretsError::NotConfigured`; the frontend sees `CredentialStatus::Missing`.
- **D-02:** Primary credential write path: `privacy_set_provider_key` IPC command. The frontend sends the key exactly once; Rust stores it in OS keychain. The frontend never receives or holds the key again. An env-var import helper may exist behind an explicit dev/test/migration flag, but production must never silently fall back to env vars.
- **D-03:** `security::command_policy` is implemented in Phase 4 as a static runtime allow-table mapping IPC command names to permitted window labels. Every IPC handler in Phase 4 calls `policy_check()` before doing work. Phase 6 can replace or augment the hardcoded table without changing `policy_check()` call sites.
- **D-04:** File selection is Rust-owned. The backend calls `tauri-plugin-dialog` to open the native OS file picker, receives the path directly from the OS, validates it, mints an opaque UUID token, and returns only the token plus safe metadata (filename, MIME type, size) to the frontend. `chat_send` must accept attachment tokens, never raw filesystem paths.
- **D-05:** File access tokens are session-scoped and in-memory only. The token map is a `Mutex<HashMap<TokenId, PathBuf>>` field in `AppState`. On app quit all tokens are dropped.
- **D-06:** Phase 4 implements `security::file_tokens` and minimal `ipc::files` commands. Wiring attachment tokens into `chat_send` is deferred to Phase 5.
- **D-07:** Three categories pass through `security::redaction` before any log or telemetry write: (1) provider API keys and credentials, (2) raw file paths, (3) all content-bearing data (prompt text, message content, attachment content).
- **D-08:** `telemetry::audit_log` writes to an append-only JSON Lines file in the app data logs directory (`<app_data>/logs/audit.log`). Schema validation and redaction run before every write.
- **D-09:** Redaction is unconditional. No debug or dev-mode escape hatch lowers content or path redaction.
- **D-10:** `ipc::privacy` exposes exactly three commands: `privacy_set_provider_key`, `privacy_get_credential_status`, `privacy_clear_provider_key`. No IPC command ever returns or exposes the key value.
- **D-11:** Phase 4 includes a minimal `SettingsSurface.svelte` implementing only credential management: API key entry (write-only), credential status display, and clear. Broader Settings UX is deferred.

### Claude's Discretion

- Keyring service name and account label format (e.g., `"desktop-ai-client"` / `"openrouter"`)
- Exact fields in safe metadata returned by `files_open_dialog` (filename, size, MIME type expected)
- JSON Lines schema for audit log entries (which fields per event type; no payload content)
- Error enum naming for `PrivacyError` and `FilesError` variants (follow `ShellError` pattern)
- In-memory token ID type (`uuid::Uuid` or a newtype wrapper)

### Deferred Ideas (OUT OF SCOPE)

- `chat_send` attachment wiring — Phase 5
- Persisted attachment metadata — Phase 5
- `security::artifact_sandbox` — Phase 5
- Full Settings experience — future settings phase
- SQLite `audit_events` table — JSON Lines is sufficient for Phase 4
- `privacy_import_from_env` helper — may be CLI flag or test helper, not an IPC command
- Phase 6 command-inventory verifier
  </user_constraints>

---

<phase_requirements>

## Phase Requirements

| ID     | Description                                                                          | Research Support                                                                                                                                           |
| ------ | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SEC-01 | Secrets stay backend-owned and are not exposed to ordinary frontend windows          | D-01/D-02/D-10: KeyringSecretStore replaces env-var backing; IPC commands never return key value; `deny_unknown_fields` structs block forbidden parameters |
| SEC-02 | File access uses opaque tokens or Rust-owned selection instead of raw frontend paths | D-04/D-05/D-06: `tauri-plugin-dialog` v2.7.1 opens picker backend-side; `security::file_tokens` mints UUID tokens; `ipc::files` returns safe metadata only |
| SEC-03 | Sensitive data is redacted before logs or telemetry                                  | D-07/D-08/D-09: `security::redaction` gates all three sensitive categories; `telemetry::audit_log` enforces redaction before JSON Lines write              |

</phase_requirements>

---

## Summary

Phase 4 implements the full privacy enforcement layer across seven Rust modules and one Svelte surface. The work is entirely backend-dominant: five security/telemetry modules implement from scratch, two IPC modules expose five new commands, and `AppState` gains a new `file_tokens` field. The Svelte component (`SettingsSurface.svelte`) is deliberately minimal — only credential write, status check, and clear.

The most critical dependency decision is the secrets backing store. The `keyring` crate has been restructured in 2025: v4.x is now "sample/demo code" only, and applications should depend on `keyring-core` v1.0.0 plus platform-specific store crates (`apple-native-keyring-store`, `windows-native-keyring-store`, `dbus-secret-service-keyring-store`). The v3.6.3 series remains usable and well-documented for simpler cross-platform needs. Research suggests pinning to `keyring-core = "1"` with explicit platform dependencies is the current recommended pattern, but `keyring = "3"` remains a viable single-crate alternative with the same `Entry::new()` / `set_password()` / `get_password()` / `delete_credential()` API.

`tauri-plugin-dialog` v2.7.1 provides the `blocking_pick_file()` / `pick_file()` builder pattern callable from a Tauri command handler via `app_handle.dialog().file()`. All five new IPC commands require entries in `main.rs generate_handler![]`, `capabilities/main.json`, and new TOML files under `src-tauri/permissions/`.

**Primary recommendation:** Implement `security::secrets` replacement first (KeyringSecretStore swaps only the backing of `SecretsState`, callers in `ipc::privacy` and `providers/` are unchanged). Then implement `security::file_tokens`, `ipc::files`, `security::redaction`, `security::command_policy`, `telemetry::audit_log`, and finally `SettingsSurface.svelte`.

---

## Architectural Responsibility Map

| Capability                 | Primary Tier                       | Secondary Tier            | Rationale                                                              |
| -------------------------- | ---------------------------------- | ------------------------- | ---------------------------------------------------------------------- |
| OS keychain read/write     | Backend (security module)          | —                         | Credentials are backend-owned; no path to frontend by invariant        |
| Credential status query    | Backend (IPC)                      | Frontend (status display) | Status (Configured/Missing) is safe to cross IPC; key value never does |
| File picker invocation     | Backend (IPC + security)           | —                         | Rust-owned selection; frontend never holds path authority              |
| File token minting         | Backend (security::file_tokens)    | —                         | Opaque token authority stays backend-scoped                            |
| Safe metadata return       | Backend (IPC) → Frontend           | —                         | filename/size/MIME carry no path authority                             |
| Redaction enforcement      | Backend (security::redaction)      | —                         | Unconditional gate before telemetry; not a frontend concern            |
| Audit log write            | Backend (telemetry::audit_log)     | —                         | JSON Lines writer with redaction gate; no SQL                          |
| Credential management UI   | Frontend (SettingsSurface.svelte)  | —                         | Write-only entry form + status display; never reads the key            |
| Command policy enforcement | Backend (security::command_policy) | —                         | Window-label allow-table replaces per-command `assert_main_window`     |

---

## Standard Stack

### Core (new additions — already in Cargo.toml unless noted)

| Library                             | Version                           | Purpose                                                                   | Status                                  |
| ----------------------------------- | --------------------------------- | ------------------------------------------------------------------------- | --------------------------------------- |
| `keyring-core`                      | `1.0.0` [VERIFIED: crates.io]     | Cross-platform credential store API (replaces `keyring 3.x` simple crate) | ADD to Cargo.toml                       |
| `apple-native-keyring-store`        | `1.0.0` [VERIFIED: crates.io]     | macOS Keychain backend for keyring-core                                   | ADD (cfg target_os = "macos")           |
| `windows-native-keyring-store`      | `1.1.0` [VERIFIED: crates.io]     | Windows Credential Manager backend                                        | ADD (cfg target_os = "windows")         |
| `dbus-secret-service-keyring-store` | `1.0.0` [VERIFIED: crates.io]     | Linux D-Bus Secret Service backend                                        | ADD (cfg linux/freebsd)                 |
| `tauri-plugin-dialog`               | `2.7.1` [VERIFIED: crates.io]     | Native OS file picker from Rust                                           | ADD to Cargo.toml + main.rs plugin init |
| `mime_guess`                        | `2.0.5` [VERIFIED: crates.io]     | File extension to MIME type (for safe metadata)                           | ADD to Cargo.toml                       |
| `uuid`                              | `1` [VERIFIED: crates.io]         | Token ID generation (`Uuid::new_v4()`)                                    | Already in Cargo.toml                   |
| `thiserror`                         | `1` [ASSUMED: Cargo.toml already] | Derive typed error enums                                                  | Already in Cargo.toml                   |
| `secrecy`                           | `0.10`                            | SecretString wrapper auto-redacts Debug/Display                           | Already in Cargo.toml                   |
| `serde_json`                        | `1`                               | JSON Lines serialization for audit log                                    | Already in Cargo.toml                   |
| `chrono`                            | `0.4`                             | Timestamps for audit log entries                                          | Already in Cargo.toml                   |

### Alternative: `keyring` v3 (simpler, fewer crates)

The `keyring = "3"` crate (currently 3.6.3) supports macOS/Windows/Linux through a single dependency with feature flags:

```toml
keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }
```

This is simpler but the upstream project now recommends applications migrate to `keyring-core`. Either approach works for Phase 4; the API is identical (`Entry::new()`, `set_password()`, `get_password()`, `delete_credential()`).

**Recommendation:** Use `keyring = "3"` (3.6.3) for simplicity in Phase 4, since the project is not yet at Phase 6 (command inventory), and the backing store can be swapped later without changing callers.

---

## Package Legitimacy Audit

> slopcheck ran against npm registry (incorrect ecosystem). All packages below are Rust crates verified against crates.io via `cargo search`.

| Package                             | Registry  | Verified Version | Source Repo                                     | Cargo Search | Disposition            |
| ----------------------------------- | --------- | ---------------- | ----------------------------------------------- | ------------ | ---------------------- |
| `keyring-core`                      | crates.io | 1.0.0            | github.com/open-source-cooperative/keyring-core | FOUND        | Approved               |
| `keyring` (v3 alt)                  | crates.io | latest 3.x       | github.com/open-source-cooperative/keyring-rs   | FOUND        | Approved (alternative) |
| `apple-native-keyring-store`        | crates.io | 1.0.0            | github.com/open-source-cooperative/             | FOUND        | Approved               |
| `windows-native-keyring-store`      | crates.io | 1.1.0            | github.com/open-source-cooperative/             | FOUND        | Approved               |
| `dbus-secret-service-keyring-store` | crates.io | 1.0.0            | github.com/open-source-cooperative/             | FOUND        | Approved               |
| `tauri-plugin-dialog`               | crates.io | 2.7.1            | github.com/tauri-apps/plugins-workspace         | FOUND        | Approved               |
| `mime_guess`                        | crates.io | 2.0.5            | github.com/abonander/mime_guess                 | FOUND        | Approved               |
| `uuid`                              | crates.io | 1.x              | already in project                              | FOUND        | Approved               |
| `thiserror`                         | crates.io | 1.x              | already in project                              | FOUND        | Approved               |

**slopcheck note:** Tool was npm-only; Rust packages were verified through `cargo search` against crates.io (correct registry). All packages confirmed present and maintained.

**Packages removed due to SLOP:** none

**Packages flagged as suspicious:** none (all are established crates with significant download counts)

---

## Architecture Patterns

### System Architecture Diagram

```
Frontend (SettingsSurface.svelte)
  |  invoke("privacy_set_provider_key", { provider, key })   [write-once, key never returned]
  |  invoke("privacy_get_credential_status", { provider })   [returns Configured | Missing]
  |  invoke("privacy_clear_provider_key", { provider })
  |  invoke("files_open_dialog")                              [returns token + safe metadata]
  |  invoke("files_read_token", { token_id })                [returns safe read result]
  v
IPC Layer (ipc::privacy, ipc::files)
  |  security::command_policy::policy_check()                [allow-table, window label]
  |  #[serde(deny_unknown_fields)] request structs           [block forbidden params]
  |  telemetry::audit_log::log_event()                       [redaction gate before write]
  |
  +---> security::secrets::KeyringSecretStore                [keyring-core Entry ops]
  |         Entry::new("desktop-ai-client", provider)
  |         .set_password(key) / .get_password() / .delete_credential()
  |
  +---> security::file_tokens                                [Mutex<HashMap<TokenId, PathBuf>>]
  |         mint_token() — app_handle.dialog().file()
  |         .blocking_pick_file() -> Option<FilePath>
  |         resolve_token() — returns safe metadata (not path)
  |
  +---> security::redaction::redact()                        [unconditional before log write]
  |
  +---> telemetry::audit_log::write_entry()                  [JSON Lines append]
            app_handle.path().app_log_dir()? / "audit.log"
            OpenOptions::new().create(true).append(true)
            serde_json::to_string(&entry)? + "\n"

AppState (app_state.rs)
  secrets: Mutex<SecretsState>            [existing — backing replaced by KeyringSecretStore]
  file_tokens: Mutex<HashMap<TokenId, PathBuf>>   [NEW — add in Phase 4]
```

### Recommended Project Structure Changes

```
src-tauri/src/
├── app_state.rs              # Add: file_tokens: Mutex<HashMap<Uuid, PathBuf>>
├── main.rs                   # Add: 5 new commands + dialog plugin init
├── security/
│   ├── secrets.rs            # Replace: SecretsState backing → KeyringSecretStore
│   ├── file_tokens.rs        # Implement: mint_token, resolve_token, revoke_token
│   ├── redaction.rs          # Implement: redact_secret, redact_path, redact_content
│   └── command_policy.rs     # Implement: CommandPolicy struct + policy_check()
├── ipc/
│   ├── privacy.rs            # Implement: 3 commands
│   └── files.rs              # Implement: 2 commands
└── telemetry/
    └── audit_log.rs          # Implement: AuditEvent, write_entry()
src-tauri/permissions/
    └── privacy.toml          # NEW: 3 privacy permission entries
    └── files.toml            # NEW: 2 files permission entries
src-tauri/capabilities/
    └── main.json             # Add: 5 new allow-* identifiers
src/lib/stores/
    └── settings.ts           # NEW: privacyStore (credential status, set, clear)
src/lib/components/surfaces/
    └── SettingsSurface.svelte  # Implement: credential management UI
```

### Pattern 1: KeyringSecretStore (replacing SecretsState backing)

**What:** Replace the env-var read in `SecretsState::default()` with OS keychain operations. The `SecretsState` struct, `get_provider_key()`, and `get_credential_status()` caller signatures are locked — only the internal backing changes.

**When to use:** On `privacy_set_provider_key` (write), `get_provider_key` (internal to providers), `privacy_clear_provider_key` (delete).

```rust
// Source: docs.rs/keyring/3.6.3 (keyring v3 simple approach)
// Cargo.toml addition:
// keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }

use keyring::Entry;

const SERVICE: &str = "desktop-ai-client";

pub fn store_provider_key(provider: &str, key: &str) -> Result<(), SecretsError> {
    let entry = Entry::new(SERVICE, provider)
        .map_err(|e| SecretsError::StorageError(e.to_string()))?;
    entry.set_password(key)
        .map_err(|e| SecretsError::StorageError(e.to_string()))
}

pub fn read_provider_key(provider: &str) -> Result<secrecy::SecretString, SecretsError> {
    let entry = Entry::new(SERVICE, provider)
        .map_err(|e| SecretsError::StorageError(e.to_string()))?;
    match entry.get_password() {
        Ok(pw) => Ok(secrecy::SecretString::new(pw.into())),
        Err(keyring::Error::NoEntry) => Err(SecretsError::NotConfigured(
            format!("{provider} API key is not configured")
        )),
        Err(e) => Err(SecretsError::StorageError(e.to_string())),
    }
}

pub fn delete_provider_key(provider: &str) -> Result<(), SecretsError> {
    let entry = Entry::new(SERVICE, provider)
        .map_err(|e| SecretsError::StorageError(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // already gone — idempotent
        Err(e) => Err(SecretsError::StorageError(e.to_string())),
    }
}
```

**Critical:** Never call `.expose_secret()` inside log macros, error strings, or IPC response fields. [CITED: secrets.rs invariant comment]

### Pattern 2: PrivacyError / FilesError typed error enums

**What:** Follow the `ShellError` pattern from `ipc/app_shell.rs` exactly. Serialize as `{ code: "SCREAMING_SNAKE_CASE", message: string }`.

```rust
// Source: [VERIFIED: src-tauri/src/ipc/app_shell.rs - established pattern]
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivacyError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("provider not supported: {0}")]
    UnsupportedProvider(String),
    #[error("credential store error: {0}")]
    CredentialStoreError(String),
    #[error("policy violation: {0}")]
    PolicyViolation(String),
}

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FilesError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("dialog cancelled by user")]
    Cancelled,
    #[error("token not found: {0}")]
    TokenNotFound(String),
    #[error("policy violation: {0}")]
    PolicyViolation(String),
    #[error("io error: {0}")]
    IoError(String),
}
```

### Pattern 3: deny_unknown_fields on IPC request structs

**What:** All Phase 4 IPC request structs use `#[serde(deny_unknown_fields)]` to prevent parameter smuggling. This is the standard approach for blocking forbidden parameters at the deserialization boundary. [CITED: CONTEXT.md D-03, adversarial hardening spec edit #36]

```rust
// Source: [CITED: docs.rs/serde - deny_unknown_fields attribute]
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetProviderKeyRequest {
    pub provider: String,
    // NOTE: no `api_key` field here — key is a top-level command param,
    // not nested in a request struct, so the frontend cannot smuggle extra fields
}
```

For commands accepting the API key directly as a `#[tauri::command]` parameter:

```rust
#[tauri::command]
pub async fn privacy_set_provider_key(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    provider: String,
    key: String,   // received once, stored, never echoed
) -> Result<(), PrivacyError> { ... }
```

### Pattern 4: File token minting with tauri-plugin-dialog

**What:** Backend opens native file picker, receives `Option<FilePath>`, extracts `PathBuf`, mints UUID token, returns safe metadata. [CITED: tauri.app/plugin/dialog/]

```rust
// Source: [CITED: tauri.app/plugin/dialog/ - blocking_pick_file]
use tauri_plugin_dialog::DialogExt;
use mime_guess::from_path;
use uuid::Uuid;

#[tauri::command]
pub async fn files_open_dialog(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<FileTokenResponse, FilesError> {
    // policy_check must run before any work
    command_policy::policy_check("files_open_dialog", window.label())?;

    // blocking_pick_file() must run on a non-Tokio thread.
    // Use tauri::async_runtime::spawn_blocking or call from a command
    // that does not need to be async (commands that don't use .await can be sync).
    // For simplicity: blocking_pick_file is safe in a Tauri command handler
    // that runs on the blocking thread pool.
    let file_path: Option<tauri_plugin_dialog::FilePath> = app_handle
        .dialog()
        .file()
        .blocking_pick_file();

    let path = match file_path {
        None => return Err(FilesError::Cancelled),
        Some(fp) => fp.into_path().map_err(|e| FilesError::IoError(e.to_string()))?,
    };

    // Extract safe metadata — no path authority crosses IPC
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let size = std::fs::metadata(&path)
        .map(|m| m.len())
        .unwrap_or(0);
    let mime_type = from_path(&path)
        .first_or_octet_stream()
        .to_string();

    // Mint opaque token
    let token_id = Uuid::new_v4();
    {
        let mut tokens = state.file_tokens.lock()
            .map_err(|e| FilesError::IoError(format!("token lock poisoned: {e}")))?;
        tokens.insert(token_id, path);
    }

    Ok(FileTokenResponse { token_id: token_id.to_string(), filename, size, mime_type })
}

#[derive(Debug, serde::Serialize)]
pub struct FileTokenResponse {
    pub token_id: String,  // opaque UUID string — carries no path authority
    pub filename: String,
    pub size: u64,
    pub mime_type: String,
}
```

**Note:** `blocking_pick_file()` must not be called on the Tokio thread. In practice, `#[tauri::command]` handlers that are `async` run on Tauri's blocking thread pool by default when they call blocking APIs, but verify this if using `tokio::spawn` internally. Safest pattern: make `files_open_dialog` a non-async command (Tauri supports sync commands) or use `tauri::async_runtime::spawn_blocking`. [ASSUMED: thread scheduling behavior — verify during implementation]

### Pattern 5: audit_log JSON Lines writer

**What:** Append-only JSON Lines file gated by redaction. Use `std::fs::OpenOptions` in append mode with `serde_json::to_string()` for each entry. [CITED: serde_json docs, D-08]

```rust
// Source: [CITED: docs.rs/serde_json + std::fs::OpenOptions]
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

#[derive(Debug, serde::Serialize)]
pub struct AuditEntry {
    pub timestamp: String,        // ISO 8601
    pub command: String,          // IPC command name — safe to log
    pub window: String,           // window label — safe to log
    pub status: String,           // "ok" | "error" | "denied"
    // NO payload content, NO secrets, NO file paths
}

pub fn write_audit_entry(
    app_handle: &tauri::AppHandle,
    entry: AuditEntry,
) -> Result<(), std::io::Error> {
    // Redaction is already implicit — AuditEntry has no content fields.
    // If additional metadata is added, pass through security::redaction first.

    let log_dir = app_handle.path().app_log_dir()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("audit.log");

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let line = serde_json::to_string(&entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    writeln!(file, "{}", line)
}
```

### Pattern 6: CommandPolicy allow-table

**What:** Static allow-table mapping command names to permitted window labels. Designed so Phase 6 can swap the backing table without changing call sites. [CITED: D-03]

```rust
// Source: [CITED: CONTEXT.md D-03 — Phase 6 extensibility requirement]
pub struct CommandPolicy {
    // Phase 4: hardcoded static table.
    // Phase 6: replace this field with a loaded command-inventory.toml.
    table: &'static [(&'static str, &'static [&'static str])],
}

impl CommandPolicy {
    pub const fn new() -> Self {
        Self {
            table: &[
                ("privacy_set_provider_key",    &["main"]),
                ("privacy_get_credential_status", &["main"]),
                ("privacy_clear_provider_key",  &["main"]),
                ("files_open_dialog",           &["main"]),
                ("files_read_token",            &["main"]),
                // Phase 2/3 commands also enforced here (migrated from assert_main_window)
                ("get_active_surface",          &["main"]),
                ("set_active_surface",          &["main"]),
                ("chat_send",                   &["main"]),
                ("chat_cancel",                 &["main"]),
                ("history_list",                &["main"]),
                ("history_get",                 &["main"]),
                ("history_delete",              &["main"]),
                ("history_search",              &["main"]),
            ],
        }
    }

    pub fn check(&self, command: &str, window_label: &str) -> Result<(), PolicyError> {
        for (cmd, allowed_windows) in self.table {
            if *cmd == command {
                if allowed_windows.contains(&window_label) {
                    return Ok(());
                } else {
                    return Err(PolicyError::UnauthorizedWindow(format!(
                        "{command} requires one of {:?}, got {window_label:?}",
                        allowed_windows
                    )));
                }
            }
        }
        // Command not in table — deny by default
        Err(PolicyError::UnknownCommand(command.to_string()))
    }
}

// Module-level singleton — compile-time const
pub static POLICY: CommandPolicy = CommandPolicy::new();

pub fn policy_check(command: &str, window_label: &str) -> Result<(), PolicyError> {
    POLICY.check(command, window_label)
}
```

### Pattern 7: AppState extension with file_tokens

**What:** Add `file_tokens: Mutex<HashMap<Uuid, PathBuf>>` to `AppState`. Follow the same field pattern as `secrets` and `active_requests`. [CITED: CONTEXT.md D-05, app_state.rs patterns]

```rust
// Source: [VERIFIED: src-tauri/src/app_state.rs - existing patterns]
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

pub struct AppState {
    pub shell: Mutex<ShellState>,
    pub active_requests: Mutex<HashMap<String, CancellationToken>>,
    pub secrets: Mutex<SecretsState>,
    // NEW in Phase 4:
    pub file_tokens: Mutex<HashMap<Uuid, std::path::PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            shell: Mutex::new(ShellState::default()),
            active_requests: Mutex::new(HashMap::new()),
            secrets: Mutex::new(SecretsState::default()),
            file_tokens: Mutex::new(HashMap::new()),  // session-scoped, no persistence
        }
    }
}
```

**Lock ordering constraint:** The existing lock order is `shell → sqlite`. The `file_tokens` lock is independent of both (it is never held while acquiring other locks). No new lock ordering constraint is introduced. [CITED: ARCHITECTURE.md §Architectural Constraints]

**IMPORTANT:** The `make_state_with_secrets()` test helper in `secrets.rs` constructs `AppState` directly and will need updating to include `file_tokens`. [VERIFIED: secrets.rs test helper]

### Pattern 8: Settings store and SettingsSurface.svelte

**What:** Follow the `historyStore` / `surfaceStore` pattern — `$state` runes, typed IPC wrappers, `normalizeIpcError()`, optimistic where appropriate. [CITED: src/lib/stores/history.ts]

```typescript
// Source: [VERIFIED: src/lib/stores/history.ts - established pattern]
import { invoke } from '@tauri-apps/api/core';

// Mirrors Rust CredentialStatus enum
export type CredentialStatus = 'CONFIGURED' | 'MISSING';

// Mirrors Rust ProviderId enum (SCREAMING_SNAKE_CASE serialization)
export type ProviderId = 'OpenRouter';

function normalizeIpcError(e: unknown): string {
	if (typeof e === 'string') return e;
	if (e && typeof e === 'object') {
		const obj = e as Record<string, unknown>;
		if (typeof obj['message'] === 'string') return obj['message'];
		if (typeof obj['code'] === 'string') return `Error: ${obj['code']}`;
	}
	return 'An unexpected error occurred.';
}

function createPrivacyStore() {
	let status = $state<CredentialStatus>('MISSING');
	let loading = $state(false);
	let error = $state<string | null>(null);

	async function loadStatus(
		provider: ProviderId = 'OpenRouter',
	): Promise<void> {
		loading = true;
		error = null;
		try {
			status = await invoke<CredentialStatus>('privacy_get_credential_status', {
				provider,
			});
		} catch (e) {
			error = normalizeIpcError(e);
		} finally {
			loading = false;
		}
	}

	async function setProviderKey(
		provider: ProviderId,
		key: string,
	): Promise<void> {
		loading = true;
		error = null;
		try {
			await invoke<void>('privacy_set_provider_key', { provider, key });
			status = 'CONFIGURED';
		} catch (e) {
			error = normalizeIpcError(e);
		} finally {
			loading = false;
		}
	}

	async function clearProviderKey(
		provider: ProviderId = 'OpenRouter',
	): Promise<void> {
		loading = true;
		error = null;
		try {
			await invoke<void>('privacy_clear_provider_key', { provider });
			status = 'MISSING';
		} catch (e) {
			error = normalizeIpcError(e);
		} finally {
			loading = false;
		}
	}

	return {
		get status() {
			return status;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		loadStatus,
		setProviderKey,
		clearProviderKey,
	};
}

export const privacyStore = createPrivacyStore();
```

**Security invariant for the UI:** The API key input must be `<input type="password">` or equivalent. The value is held in a local `$state` variable only long enough to call `setProviderKey()`, then immediately cleared. The store never caches the key value. [CITED: D-02, adversarial spec invariant #1]

### Pattern 9: Permissions and capabilities for new commands

**What:** Each new command needs a TOML permission file under `src-tauri/permissions/` and an entry in `capabilities/main.json`. Follow the pattern in `permissions/history.toml`. [CITED: v2.tauri.app/security/permissions/ + established project pattern]

```toml
# src-tauri/permissions/privacy.toml

[[permission]]
identifier = "allow-privacy-set-provider-key"
description = "Permits writing a provider API key to the OS keychain from the main window. Key is stored backend-side; never returned to frontend."

[permission.commands]
allow = ["privacy_set_provider_key"]

[[permission]]
identifier = "allow-privacy-get-credential-status"
description = "Permits querying whether a provider credential is configured. Returns status only, never the key value."

[permission.commands]
allow = ["privacy_get_credential_status"]

[[permission]]
identifier = "allow-privacy-clear-provider-key"
description = "Permits deleting a provider API key from the OS keychain."

[permission.commands]
allow = ["privacy_clear_provider_key"]
```

```toml
# src-tauri/permissions/files.toml

[[permission]]
identifier = "allow-files-open-dialog"
description = "Permits opening the native OS file picker backend-side. Returns a token and safe metadata; never returns a raw path."

[permission.commands]
allow = ["files_open_dialog"]

[[permission]]
identifier = "allow-files-read-token"
description = "Permits reading file content identified by a backend-issued opaque token."

[permission.commands]
allow = ["files_read_token"]
```

```json
// additions to capabilities/main.json permissions array:
"allow-privacy-set-provider-key",
"allow-privacy-get-credential-status",
"allow-privacy-clear-provider-key",
"allow-files-open-dialog",
"allow-files-read-token"
```

### Anti-Patterns to Avoid

- **Never log or echo the API key value.** Not in error messages, not in audit log entries, not in eprintln! debug output. Use `secrecy::SecretString` throughout the internals. [CITED: CONTEXT.md §Specific Ideas]
- **Never accept a `raw_path` parameter in any IPC command.** Frontend sends only tokens; backend mints only tokens. [CITED: adversarial spec invariant #13]
- **Never call `.expose_secret()` inside a `format!()`, `println!()`, or log macro.** The `SecretString` wrapper exists to prevent exactly this. [CITED: secrets.rs invariant comment]
- **Never hold the `file_tokens` Mutex lock across an `await` point.** Acquire, insert/remove, drop before returning or awaiting. [CITED: ARCHITECTURE.md lock ordering rules]
- **Never fall back silently to env-var in production.** If keychain access fails, return `SecretsError::NotConfigured`. [CITED: D-01]

---

## Don't Hand-Roll

| Problem                       | Don't Build                     | Use Instead                            | Why                                                                                   |
| ----------------------------- | ------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------- |
| OS keychain storage           | Custom encrypted file, env vars | `keyring = "3"` / `keyring-core = "1"` | Platform keychain handles encryption, ACL enforcement, and unlock/lock lifecycle      |
| Secure string memory clearing | Manual `memset` on String       | `secrecy::SecretString`                | Already in project; handles Debug/Display redaction and zeroize-on-drop automatically |
| File extension to MIME type   | Custom match table              | `mime_guess = "2"`                     | Static 4,900+ entry table; 5M+ weekly downloads; no false negatives for common types  |
| Native file dialog invocation | Custom IPC protocol             | `tauri-plugin-dialog`                  | Tauri-maintained plugin; integrates with AppHandle state lifecycle correctly          |
| UUID token generation         | Custom RNG                      | `uuid::Uuid::new_v4()`                 | Already in project; cryptographically random via OS entropy                           |
| Typed error serialization     | Custom enum `impl Serialize`    | `thiserror` + `serde(tag)`             | Already in project; see `ShellError` pattern                                          |

**Key insight:** This phase is almost entirely plumbing between well-tested crates. The security value comes from the architecture (what crosses IPC), not from novel cryptography. Don't invent new crypto primitives.

---

## Common Pitfalls

### Pitfall 1: keyring API method name `delete_password` vs `delete_credential`

**What goes wrong:** Calling `entry.delete_password()` which was the v2 API. v3+ renamed it to `delete_credential()`.
**Why it happens:** Training data and many Stack Overflow answers reference the older API.
**How to avoid:** Use `entry.delete_credential()`. [VERIFIED: docs.rs/keyring/3.6.3]
**Warning signs:** Compiler error "no method named `delete_password`" — easy to catch.

### Pitfall 2: keyring crate v4 is not the right crate to use

**What goes wrong:** Adding `keyring = "4"` and discovering the v4 crate is "sample/demo code" only per upstream README.
**Why it happens:** `cargo search keyring` returns `keyring = "4.0.1"` as the top result.
**How to avoid:** Either pin `keyring = "3"` explicitly (3.6.3), or use `keyring-core = "1"` with platform store crates. [VERIFIED: github.com/open-source-cooperative/keyring-rs/releases — v4 README states "Apps should not depend on this release"]
**Warning signs:** Compilation may succeed but runtime behavior may be mock/sample.

### Pitfall 3: blocking_pick_file() on the Tokio async thread

**What goes wrong:** Calling `app_handle.dialog().file().blocking_pick_file()` inside an `async #[tauri::command]` that is running on a Tokio async worker thread. This can deadlock on some platforms (macOS UI thread requirement).
**Why it happens:** Tauri async commands look like normal async fns; the thread constraint is easy to miss.
**How to avoid:** Use either (a) a sync (non-async) `#[tauri::command]` function, which Tauri runs on a blocking thread pool, or (b) `tauri::async_runtime::spawn_blocking(|| ...)`. [ASSUMED: platform thread constraint — verify against Tauri v2 docs during implementation]
**Warning signs:** Dialog freezes or never returns on macOS; works on Linux/Windows where there is no UI thread constraint.

### Pitfall 4: secrets.rs test helper requires AppState update

**What goes wrong:** Existing `make_state_with_secrets()` test helper in `secrets.rs` constructs `AppState` directly. After adding `file_tokens` field to `AppState`, this helper will fail to compile.
**Why it happens:** Struct literal initialization is exhaustive in Rust.
**How to avoid:** Update the helper to include `file_tokens: Mutex::new(HashMap::new())` in the same task that adds the field to `AppState`. [VERIFIED: secrets.rs lines 100-108]
**Warning signs:** Compile error at test helper instantiation site.

### Pitfall 5: `#[serde(tag, content)]` incompatibility with `deny_unknown_fields`

**What goes wrong:** Combining `#[serde(tag = "code", content = "message")]` (on error enums for responses) with `#[serde(deny_unknown_fields)]` (on request structs for deserialization). These serve different types and must not be combined on the same struct.
**Why it happens:** Serde issue #2666 documents that `tag` + `deny_unknown_fields` on the same type has known serialization problems.
**How to avoid:** `deny_unknown_fields` goes on **request/input structs** (deserializing from frontend). `tag + content + rename_all` goes on **error/response enums** (serializing to frontend). Never combine both on the same type. [CITED: github.com/serde-rs/serde/issues/2666]
**Warning signs:** Runtime serde deserialization panics or unexpected behavior; may not be caught at compile time.

### Pitfall 6: Audit log path not created before first write

**What goes wrong:** `OpenOptions::new().append(true).open()` fails with `NotFound` if the `logs/` subdirectory doesn't exist.
**Why it happens:** `app_handle.path().app_log_dir()` returns the path but doesn't create it.
**How to avoid:** Call `std::fs::create_dir_all(&log_dir)` before the first `OpenOptions::new()` call. [CITED: main.rs setup hook pattern — same pattern used for `app_data_dir`]
**Warning signs:** First audit log write returns `ErrorKind::NotFound`; subsequent writes succeed once directory exists.

### Pitfall 7: Svelte API key input leaking to DOM

**What goes wrong:** Rendering the API key value in a `<span>` or `aria-label` for status feedback, making it readable from DevTools.
**Why it happens:** Wanting to confirm the key was saved by showing a preview.
**How to avoid:** The UI must never receive the key value back from the backend. Status feedback is always `CONFIGURED | MISSING`. The input is `<input type="password">` and the local Svelte `$state` holding the typed key is cleared immediately after `setProviderKey()` returns. [CITED: D-02, adversarial spec invariant #1]

---

## Code Examples

### keyring v3 Entry API (verified)

```rust
// Source: [VERIFIED: docs.rs/keyring/3.6.3/keyring/struct.Entry.html]
use keyring::{Entry, Error};

// Create entry (service name, account/user label)
let entry = Entry::new("desktop-ai-client", "openrouter")?;

// Write
entry.set_password("sk-or-v1-...")?;

// Read
match entry.get_password() {
    Ok(pw) => { /* pw is a String */ }
    Err(Error::NoEntry) => { /* not configured */ }
    Err(e) => { /* platform failure */ }
}

// Delete
entry.delete_credential()?;  // v3+ API (NOT delete_password)
```

### tauri-plugin-dialog FilePath extraction

```rust
// Source: [VERIFIED: docs.rs/tauri-plugin-dialog/latest - FileDialogBuilder]
// FilePath::into_path() converts to PathBuf
use tauri_plugin_dialog::{DialogExt, FilePath};

let fp: Option<FilePath> = app_handle.dialog().file().blocking_pick_file();
if let Some(file_path) = fp {
    match file_path.into_path() {
        Ok(pb) => { /* pb is a PathBuf */ }
        Err(_) => { /* URI path (Android) — not supported in desktop phase */ }
    }
}
```

### PathResolver for app log dir (Tauri v2)

```rust
// Source: [VERIFIED: docs.rs/tauri/2.11.2/tauri/path/struct.PathResolver.html]
// app_log_dir() returns Result<PathBuf>
// Linux: ~/.local/share/<bundle_id>/logs
// macOS: ~/Library/Logs/<bundle_id>
// Windows: %LOCALAPPDATA%\<bundle_id>\logs
let log_dir: std::path::PathBuf = app_handle.path().app_log_dir()?;
```

### IPC command registration (5 new commands)

```rust
// Source: [VERIFIED: src-tauri/src/main.rs - established pattern]
// In main.rs generate_handler![] — add to existing list:
ipc::privacy::privacy_set_provider_key,
ipc::privacy::privacy_get_credential_status,
ipc::privacy::privacy_clear_provider_key,
ipc::files::files_open_dialog,
ipc::files::files_read_token,

// In main.rs setup: add dialog plugin
.plugin(tauri_plugin_dialog::init())
```

---

## State of the Art

| Old Approach                       | Current Approach                                         | When Changed              | Impact                                                               |
| ---------------------------------- | -------------------------------------------------------- | ------------------------- | -------------------------------------------------------------------- |
| `keyring` all-in-one crate         | `keyring-core` + separate platform stores                | keyring v4 release (2025) | More crates but official "correct" API going forward; v3 still works |
| `assert_main_window()` per-handler | `command_policy::policy_check()` centralized allow-table | Phase 4                   | Phase 6 can swap backing table without touching command handlers     |
| `SecretsState` backed by env vars  | `KeyringSecretStore` via OS keychain                     | Phase 4                   | Production fail-closed; no silent fallback                           |
| `telemetry::audit_log()` stub      | Append-only JSON Lines with redaction gate               | Phase 4                   | Machine-readable audit trail for release evidence                    |

**Deprecated/outdated in this codebase:**

- `SecretsState::default()` reading from `OPENROUTER_API_KEY` env var: replaced in Phase 4 for production, kept behind dev flag
- Per-command `assert_main_window()` calls: superseded by `command_policy::policy_check()` (existing commands may keep their per-handler check as belt-and-suspenders, or be migrated)

---

## Assumptions Log

| #   | Claim                                                                                                           | Section              | Risk if Wrong                                                                       |
| --- | --------------------------------------------------------------------------------------------------------------- | -------------------- | ----------------------------------------------------------------------------------- |
| A1  | `blocking_pick_file()` is safe to call from a sync `#[tauri::command]` without deadlocking on macOS             | Pitfall 3, Pattern 4 | Dialog freeze on macOS; need to use `spawn_blocking` instead                        |
| A2  | `keyring = "3"` (3.6.3) is the best single-crate choice for Phase 4 over `keyring-core = "1"`                   | Standard Stack       | Could pick keyring-core if planner prefers forward-compatibility                    |
| A3  | `thiserror = "1"` is compatible with the new error enum patterns (v1 vs v2 API diff is minor for this use case) | Pattern 2            | If project migrates to thiserror v2, `#[error]` attribute syntax changes slightly   |
| A4  | The `privacyStore` TypeScript store uses `SCREAMING_SNAKE_CASE` for `CredentialStatus` values                   | Pattern 8            | If Rust `CredentialStatus` serde serialization changes, TypeScript types must match |

---

## Open Questions

1. **keyring v3 or keyring-core v1?**
   - What we know: Both work; v3 is simpler (one crate); keyring-core is the upstream-preferred future path
   - What's unclear: Whether Phase 6 release evidence will require newer ecosystem
   - Recommendation: Use `keyring = "3"` for Phase 4 simplicity; the backing store is isolated behind the `security::secrets` module boundary

2. **sync vs async command for `files_open_dialog`**
   - What we know: `blocking_pick_file()` is designed for non-main-thread blocking calls; Tauri async commands run on a thread pool
   - What's unclear: Whether macOS requires `spawn_blocking` wrapper or whether Tauri's blocking thread pool is sufficient
   - Recommendation: Start with a sync (non-async) `#[tauri::command]` for `files_open_dialog`, which Tauri routes to a blocking thread pool

3. **`command_policy` migration scope**
   - What we know: D-03 requires `policy_check()` on all Phase 4 handlers; existing handlers have per-command `assert_main_window()`
   - What's unclear: Whether Phase 4 should migrate all existing commands to `policy_check()` or only add it to new commands
   - Recommendation: Add `policy_check()` to all 5 new Phase 4 commands; leave existing commands with their current `assert_main_window()` calls for now (belt-and-suspenders, harmless)

---

## Environment Availability

| Dependency                  | Required By                  | Available         | Version             | Fallback                       |
| --------------------------- | ---------------------------- | ----------------- | ------------------- | ------------------------------ |
| macOS Keychain              | KeyringSecretStore (macOS)   | ✓ (OS-provided)   | —                   | EnvSecretStore behind dev flag |
| Windows Credential Manager  | KeyringSecretStore (Windows) | ✓ (OS-provided)   | —                   | EnvSecretStore behind dev flag |
| Linux SecretService (D-Bus) | KeyringSecretStore (Linux)   | Depends on distro | —                   | EnvSecretStore behind dev flag |
| `cargo` (Rust toolchain)    | Building new crates          | ✓                 | verified in project | —                              |
| `uuid` crate                | Token minting                | ✓                 | v1 in Cargo.toml    | —                              |
| `chrono` crate              | Audit log timestamps         | ✓                 | v0.4 in Cargo.toml  | —                              |

**Missing dependencies with no fallback:** None — all new crates are additive.

**Missing dependencies with fallback:**

- Linux SecretService: If `dbus-secret-service-keyring-store` is unavailable at runtime, `SecretsState` must return `NotConfigured`; the `EnvSecretStore` flag provides the dev fallback.

---

## Validation Architecture

### Test Framework

| Property           | Value                                                                              |
| ------------------ | ---------------------------------------------------------------------------------- |
| Framework          | Rust built-in (`cargo test`)                                                       |
| Config file        | `src-tauri/Cargo.toml`                                                             |
| Quick run command  | `cargo test --manifest-path src-tauri/Cargo.toml -p desktop-ai-client -- security` |
| Full suite command | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`                    |

### Phase Requirements → Test Map

| Req ID | Behavior                                                                   | Test Type | Automated Command                                                                    | File Exists?                |
| ------ | -------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------ | --------------------------- |
| SEC-01 | `get_provider_key` returns `NotConfigured` when keychain empty             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- secrets::tests`                  | ✅ (existing in secrets.rs) |
| SEC-01 | `get_provider_key` returns key after `store_provider_key` (mocked keyring) | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- secrets::tests`                  | ❌ Wave 0                   |
| SEC-01 | `privacy_set_provider_key` never exposes key in IPC response               | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::privacy::tests`             | ❌ Wave 0                   |
| SEC-01 | `PrivacyError` serializes with SCREAMING_SNAKE_CASE code field             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::privacy::tests`             | ❌ Wave 0                   |
| SEC-01 | `command_policy` denies unknown commands                                   | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- security::command_policy::tests` | ❌ Wave 0                   |
| SEC-01 | `command_policy` denies wrong window label                                 | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- security::command_policy::tests` | ❌ Wave 0                   |
| SEC-02 | `files_open_dialog` returns `Cancelled` when no file selected              | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::files::tests`               | ❌ Wave 0                   |
| SEC-02 | Token resolves to path after mint, fails after revoke                      | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- security::file_tokens::tests`    | ❌ Wave 0                   |
| SEC-02 | `FilesError` serializes with SCREAMING_SNAKE_CASE code field               | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::files::tests`               | ❌ Wave 0                   |
| SEC-03 | `redact_path` replaces path with `[REDACTED]`                              | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- security::redaction::tests`      | ❌ Wave 0                   |
| SEC-03 | `redact_secret` replaces API key with `[REDACTED]`                         | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- security::redaction::tests`      | ❌ Wave 0                   |
| SEC-03 | Audit log entry does not contain path or secret                            | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- telemetry::audit_log::tests`     | ❌ Wave 0                   |
| SEC-03 | `AuditEntry` serializes to valid JSON (parseable)                          | unit      | `cargo test --manifest-path src-tauri/Cargo.toml -- telemetry::audit_log::tests`     | ❌ Wave 0                   |

### Sampling Rate

- **Per task commit:** `cargo test --manifest-path src-tauri/Cargo.toml -- <module>::tests`
- **Per wave merge:** `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `src-tauri/src/security/secrets.rs` — test for keyring-backed store (using mock keyring in test cfg)
- [ ] `src-tauri/src/security/file_tokens.rs` — tests for `mint_token`, `resolve_token`, `revoke_token`
- [ ] `src-tauri/src/security/redaction.rs` — tests for all three redaction categories
- [ ] `src-tauri/src/security/command_policy.rs` — tests for allow-table enforcement
- [ ] `src-tauri/src/ipc/privacy.rs` — error serialization tests (no Tauri runtime needed)
- [ ] `src-tauri/src/ipc/files.rs` — error serialization tests + FilesError variants
- [ ] `src-tauri/src/telemetry/audit_log.rs` — JSON Lines structure + redaction verification
- [ ] `src-tauri/src/app_state.rs` — update `make_state_with_secrets()` helper to include `file_tokens`

**Note on keyring mock:** For CI testing without a real OS keychain, `keyring = "3"` supports a mock credential store when the platform native stores are not compiled. Alternatively, use a `#[cfg(test)]` mock trait. The decision is left to the planner but must be addressed in Wave 0.

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category                          | Applies | Standard Control                                                                                    |
| -------------------------------------- | ------- | --------------------------------------------------------------------------------------------------- |
| V2 Authentication (credential storage) | Yes     | `keyring-core` / `keyring` OS keychain; `secrecy::SecretString` in memory                           |
| V3 Session Management                  | Partial | File tokens are session-scoped in-memory `HashMap`; no persistence                                  |
| V4 Access Control                      | Yes     | `command_policy::policy_check()` — window-label allow-table on all IPC commands                     |
| V5 Input Validation                    | Yes     | `#[serde(deny_unknown_fields)]` on all request structs; `FilePath::into_path()` for path validation |
| V6 Cryptography                        | Partial | No custom crypto — OS keychain handles encryption; `Uuid::new_v4()` for token entropy               |
| V9 Communications                      | No      | No new network paths in this phase                                                                  |

### Known Threat Patterns for this Stack

| Pattern                                         | STRIDE                 | Standard Mitigation                                                                     |
| ----------------------------------------------- | ---------------------- | --------------------------------------------------------------------------------------- |
| Frontend reading API key via IPC response       | Information Disclosure | `privacy_*` commands never return key; `CredentialStatus` is a safe enum                |
| Smuggling extra parameters to bypass policy     | Tampering              | `#[serde(deny_unknown_fields)]` on all input structs                                    |
| Frontend supplying raw path to file read        | Elevation of Privilege | `ipc::files` accepts only backend-issued UUID tokens; no raw path parameter             |
| Unauthorized window calling privileged commands | Spoofing               | `command_policy::policy_check()` enforces window label before any work                  |
| Key value appearing in audit log                | Information Disclosure | `AuditEntry` struct has no content fields; `security::redaction` gate before writes     |
| API key leaking through error messages          | Information Disclosure | `SecretsError::StorageError(String)` — message must not include key value in call sites |

### Non-Negotiable Invariants (from adversarial spec)

1. No secret read path to the frontend — confirmed by D-10 (status-only IPC), D-02 (write-once), and absence of `key` field in any response type.
2. No arbitrary frontend path-to-read bridge — confirmed by D-04 (Rust-owned picker), D-06 (token-only API).
3. No unallowlisted Tauri command surface — confirmed by D-03 (command_policy allow-table) + 5 new permission TOML entries + `main.json` additions.

---

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/ipc/app_shell.rs` — canonical IPC patterns verified directly from codebase
- `src-tauri/src/security/secrets.rs` — Phase 2 implementation; locked caller interface verified
- `src-tauri/src/app_state.rs` — AppState structure, lock ordering constraints
- `src-tauri/Cargo.toml` — existing dependencies confirmed
- `src-tauri/capabilities/main.json`, `permissions/*.toml` — established capability/permission pattern
- `src/lib/stores/surface.ts`, `src/lib/stores/history.ts` — established Svelte store patterns
- [docs.rs/tauri/2.11.2/tauri/path/struct.PathResolver.html](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html) — `app_log_dir()` API
- [docs.rs/keyring/3.6.3](https://docs.rs/keyring/3.6.3/keyring/struct.Entry.html) — `Entry::new()`, `set_password()`, `get_password()`, `delete_credential()` signatures
- [docs.rs/tauri-plugin-dialog/latest/FileDialogBuilder](https://docs.rs/tauri-plugin-dialog/latest/tauri_plugin_dialog/struct.FileDialogBuilder.html) — `blocking_pick_file()`, `pick_file()`, return types
- [v2.tauri.app/security/permissions/](https://v2.tauri.app/security/permissions/) — TOML permission file format

### Secondary (MEDIUM confidence)

- [github.com/open-source-cooperative/keyring-rs/releases](https://github.com/open-source-cooperative/keyring-rs/releases) — v4 deprecation notice; v3 vs keyring-core recommendation
- [docs.rs/keyring-core/1.0.0](https://docs.rs/keyring-core/latest/keyring_core/struct.Entry.html) — `keyring-core` v1 Entry API
- [crates.io/crates/mime_guess](https://crates.io/crates/mime_guess) — mime_guess 2.0.5 verified

### Tertiary (LOW confidence — marked [ASSUMED] where used)

- [deepwiki.com/tauri-apps/tauri/3.1-command-system](https://deepwiki.com/tauri-apps/tauri/3.1-command-system) — blocking thread pool behavior for sync commands

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — all crates verified against crates.io; API methods verified against docs.rs
- Architecture: HIGH — patterns extracted directly from existing codebase (`app_shell.rs`, `app_state.rs`, `secrets.rs`, store files)
- Pitfalls: HIGH (1, 4, 5, 6) / MEDIUM (2, 3, 7) — compile-time catches are HIGH; runtime platform threading is MEDIUM
- Test map: HIGH — test locations follow existing `cargo test` patterns

**Research date:** 2026-06-15
**Valid until:** 2026-09-15 (stable ecosystem; keyring-core is new but API is stable)
