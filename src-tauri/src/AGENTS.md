# src-tauri/src

Shared backend module tree for IPC, providers, security, storage, telemetry, and app state.

## Read First

Before editing code here, read:
1. `../../AGENTS.md`
2. `../AGENTS.md`
3. The nearest child `AGENTS.md` for the subsystem you are changing

## Purpose

Owns the Rust modules that implement backend behavior. Does not own Tauri configuration, frontend UI, or repo-wide docs.

## Entry Points

- `main.rs` - Crate entrypoint
- `app_state.rs` - Shared runtime state
- `ipc/mod.rs` - Frontend-facing command surface
- `providers/mod.rs` - Provider capability and routing layer
- `security/mod.rs` - Secrets, redaction, command policy, sandboxing
- `storage/mod.rs` - Persistence, retention, and migration helpers
- `telemetry/mod.rs` - Audit logging and release evidence

## Contracts & Invariants

- Keep module boundaries narrow; each leaf directory owns one concern.
- Route changes through the appropriate leaf node instead of cross-importing behavior across subsystems.
- `AppState` owns exactly three things: the shell preference cache, `active_requests: Mutex<HashMap<String, CancellationToken>>` (in-flight chat streams), and `file_tokens` (opaque path map). It must never expose provider credentials, raw file paths, or prompt content across IPC (`app_state.rs:7-15`).
- `AppState.file_tokens`'s lock is independent of the shell/sqlite lock ordering and must never be held across an `.await` (`app_state.rs:28-33`).
- Adding a new `Surface` variant requires a matching migration in `storage/migrations.rs` — the enum and the migration set are coupled (`app_state.rs:77-79`).
- If a change touches privacy, command execution, or persistence policy, check the corresponding leaf node before editing.

## Pitfalls

- `AppState.secrets: Mutex<SecretsState>` is dead code as of commit `c7fffd1`. `security::secrets` now reads/writes the OS keychain directly via free functions (`store_provider_key`, `read_provider_key`, `get_provider_key`) that take no `AppState` parameter — no call site outside `app_state.rs`'s own `Default` impl reads `state.secrets`. Don't thread `AppState` through new secrets call sites; call the free functions directly.
- `Default for AppState` still unconditionally reads `OPENROUTER_API_KEY` from the process environment at every startup as part of constructing the now-unused `SecretsState` — a pointless lookup, not a real config path (`app_state.rs:45-63`).

## Related Context

- IPC commands: `ipc/AGENTS.md`
- Provider routing: `providers/AGENTS.md`
- Security and redaction: `security/AGENTS.md`
- Storage and retention: `storage/AGENTS.md`
- Telemetry and evidence: `telemetry/AGENTS.md`
