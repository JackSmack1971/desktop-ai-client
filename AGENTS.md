# Desktop AI Client

Monorepo for the desktop client shell, the Tauri backend, and supporting docs and tests.

## Intent Layer

Read the nearest `AGENTS.md` before editing code in a subdirectory.

- `src-tauri/AGENTS.md` - Rust/Tauri backend and nested backend modules

## Global Invariants

- Treat `docs/` and `.planning/` as the contract while the app is still scaffolded.
- Keep privacy boundaries explicit; redact sensitive data before logging or telemetry.
- Keep backend-owned concerns backend-owned: command policy, provider routing, storage, and telemetry stay out of the renderer.
- Prefer small, local AGENTS nodes when a subsystem has distinct ownership or invariants.

## Working Rules

- Prefer the smallest correct change, update docs when behavior or boundaries change, and verify with the narrowest meaningful command set.
