# .claude Map

This directory is the Claude Code operating layer for `desktop-ai-client`.

## Structure

- `CLAUDE.md` at the repo root holds the top-level operating contract.
- `settings.json` defines shared permissions, confirmations, and hooks.
- `rules/` holds path-scoped boundary rules.
- `agents/` holds isolated high-context roles.
- `commands/` holds user-invoked slash commands.
- `skills/` holds reusable procedures that would otherwise bloat `CLAUDE.md`.
- `workflows/` holds deterministic orchestration contracts and report emitters.
- `hooks/` holds lifecycle guardrails and shared helper code.
- `output-styles/` holds response overlays for recurring report shapes.
- `security-patterns.yaml` and `claude-security-guidance.md` support security review and hook-side scanning.

## Read Order

1. `../CLAUDE.md`
2. `settings.json`
3. `rules/README.md`
4. `agents/README.md`
5. `skills/README.md`
6. `commands/README.md`
7. `workflows/README.md`
8. `hooks/README.md`
9. `output-styles/README.md`

