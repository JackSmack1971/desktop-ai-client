# Architecture

## System shape

The repository is organized as a desktop client shell with a Rust/Tauri backend and an unimplemented frontend surface. The backend is split into named subsystems for IPC, providers, security, storage, and telemetry, with a tiny shared `AppState`.

## Module boundaries

- `main.rs` is only a bootstrap stub for now
- `app_state.rs` holds minimal shared state
- `ipc/` is the frontend-facing command boundary
- `providers/` owns capability detection and routing
- `security/` owns redaction, secrets, command policy, and sandboxing
- `storage/` owns persistence, migrations, backups, and retention
- `telemetry/` owns audit logging and release evidence

## Document-driven architecture

The `docs/` directory is the strongest source of architectural intent. It defines:

- the memory-loop concept for long-running agent behavior
- privacy boundaries and threat model concerns
- provider routing intent
- command inventory expectations
- prompt and coordination guidance for agents

## Runtime flow today

There is no real runtime flow yet. The current executable entrypoint is a scaffold, and the module tree only contains placeholder functions. The architecture is therefore a boundary map and not a working system.

## Build order implied by the tree

1. Establish the crate and frontend package manifests
2. Wire the Tauri runtime and application state
3. Implement IPC command registration
4. Add provider routing and transport handling
5. Add storage, migrations, and retention
6. Add redaction, secrets, and command policy enforcement
7. Add telemetry and release evidence
