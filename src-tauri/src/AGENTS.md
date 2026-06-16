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
- Keep shared state small and explicit.
- If a change touches privacy, command execution, or persistence policy, check the corresponding leaf node before editing.

## Related Context

- IPC commands: `ipc/AGENTS.md`
- Provider routing: `providers/AGENTS.md`
- Security and redaction: `security/AGENTS.md`
- Storage and retention: `storage/AGENTS.md`
- Telemetry and evidence: `telemetry/AGENTS.md`
