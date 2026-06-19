# src/lib/components/AGENTS.md

This subtree owns frontend presentation components.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/privacy-boundaries.md`
3. `../stores/AGENTS.md`
4. `../api/AGENTS.md`

## Purpose

Owns Svelte UI composition, layout, and surface rendering for the desktop shell. Does not own persistence, IPC implementations, or backend policy.

## Rules

- Keep components presentational; move stateful logic into stores.
- Use the frontend API wrappers in `src/lib/api/` instead of importing Tauri APIs directly.
- Keep surface-specific behavior local to the matching component subtree.
- Preserve privacy boundaries in UI text, logging, and previews.

