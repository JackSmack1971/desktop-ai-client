# Rules

Rules stay small and path-scoped. Load the narrowest rule file that covers the change.

## Rule Stack

- `constitutional-agent-engineering-rules`: global agent behavior and handoff discipline
- `frontend`: UI/UX standards, component patterns, and frontend build rules
- `backend`: API design, provider routing, storage, IPC, and backend-owned behavior
- `security`: secret handling, PII, exposure control, and security-sensitive config
- `testing`: test-driven development, coverage rules, and verification expectations
- `surgical-density`: density, scope control, and change-size discipline

## Composition Order

1. Start with `frontend` and `testing`.
2. Add `security` for anything that can expose secrets, raw paths, prompt content, or privileged config.
3. Add `backend` for provider selection, streaming, IPC, storage, or Tauri config work.
