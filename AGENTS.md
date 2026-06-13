# Desktop AI Client

Monorepo for the desktop client shell, the Tauri backend, and supporting docs and tests.

## Intent Layer

Before editing code in a subdirectory, read that directory's `AGENTS.md` first.

- `src-tauri/AGENTS.md` - Rust/Tauri backend and its nested backend modules

## Global Invariants

- Keep privacy boundaries explicit; redact sensitive data before logging or telemetry.
- Keep command policy, provider routing, storage, and telemetry concerns separated.
- Treat docs in `docs/` as the source of truth for boundaries and behavior contracts.
- Prefer small, local AGENTS nodes when a subsystem has distinct ownership or invariants.

## Project Snapshot

- Project: Desktop AI Client
- Current state: brownfield scaffold with docs-defined architecture and placeholder backend/frontend modules
- Core value: keep local history and file ownership private while safely routing AI inference and generated artifacts
- Next step: run `$gsd-plan-phase 1` after project initialization is complete
