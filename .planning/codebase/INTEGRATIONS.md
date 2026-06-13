# Integrations

## Summary

The codebase is structured around integrations that have not been implemented yet. The current files define intended boundaries for external providers, local storage, telemetry, and the Tauri runtime bridge.

## Planned or implied integrations

- Tauri runtime bridge between frontend and Rust backend
- LLM/provider routing through `src-tauri/src/providers`
- OpenRouter support signaled by `openrouter.rs`
- Server-sent events transport support signaled by `sse.rs`
- SQLite persistence and migrations in `src-tauri/src/storage`
- Audit logging and release evidence in `src-tauri/src/telemetry`
- Command policy, redaction, secrets, and sandboxing in `src-tauri/src/security`

## Integration status

- No live provider API integration was implemented in the checked-in source
- No database schema or migration content was implemented yet
- No telemetry sink or export pipeline was implemented yet
- No IPC command registration was implemented yet

## External risk

The integration surface is broad relative to the amount of code that exists. That creates a drift risk: docs describe intended behavior more completely than the code currently enforces it.
