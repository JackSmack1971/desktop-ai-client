# Structure

## Top-level layout

- `docs/` - product and agent-facing design context
- `src-tauri/` - Rust/Tauri backend scaffold
- `src/` - frontend scaffold
- `tests/` - empty test areas and fixtures
- `scripts/` - placeholder for future tooling
- `.github/` - workflow placeholder

## Backend tree

- `src-tauri/src/main.rs` - entrypoint stub
- `src-tauri/src/app_state.rs` - shared state stub
- `src-tauri/src/ipc/` - command handlers
- `src-tauri/src/providers/` - provider routing and transport
- `src-tauri/src/security/` - secrets and policy controls
- `src-tauri/src/storage/` - persistence and retention
- `src-tauri/src/telemetry/` - audit and evidence

## Frontend tree

- `src/app.html` exists as a placeholder shell entry
- `src/lib/` exists only as directories with `.gitkeep`
- `src/routes/` exists only as a placeholder directory

## Test tree

- `tests/rust/` - Rust test placeholder
- `tests/e2e/` - end-to-end test placeholder
- `tests/security/` - security test placeholder
- `tests/fixtures/` - fixture buckets for adversarial and drift scenarios

## Structural note

The tree is intentionally narrow and modular, but the actual application code has not been filled in yet. Most files are placeholders whose main value is signaling ownership boundaries.
