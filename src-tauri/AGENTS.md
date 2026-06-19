# src-tauri

Tauri crate boundary for the desktop backend, build/runtime config, and backend persistence setup.

## Read First

Before editing code here, read:

1. `../AGENTS.md`
2. `../docs/architecture.md`
3. `../docs/privacy-boundaries.md`
4. `../docs/threat-model.md`
5. `src/AGENTS.md`

## Purpose

Owns the Rust backend crate, Tauri configuration, and migration assets. Does not own frontend UI code or top-level docs.

## Entry Points

- `src/main.rs` - Backend bootstrap entrypoint
- `src/app_state.rs` - Shared runtime state
- `tauri.conf.json` - Tauri app configuration
- `migrations/` - Backend migration assets

## Contracts & Invariants

- Keep backend config and migration changes explicit and reviewable.
- Keep Tauri bootstrap thin; push behavior into named backend modules.
- Do not duplicate frontend concerns here.
- Keep cross-cutting backend rules in `src/AGENTS.md` or the relevant leaf node.

## Related Context

- Shared backend modules: `src/AGENTS.md`
