# src/lib/api/AGENTS.md

This subtree owns the frontend IPC wrapper boundary.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/privacy-boundaries.md`
3. `../../../docs/command-inventory.md`

## Purpose

Owns typed frontend wrappers around Tauri `invoke()` / `Channel` calls and shared IPC error normalization. Does not own UI state, rendering, or backend policy.

## Rules

- Keep Tauri calls isolated here; other frontend code should import these wrappers instead of calling `@tauri-apps/api/core` directly.
- Keep payload shapes explicit and typed.
- Keep user-facing error normalization shared.
- Do not add command policy or routing logic here; those stay backend-owned.

