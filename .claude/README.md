# .claude Map

This directory is the Claude Code operating layer for `desktop-ai-client`.

## Structure

- `.claude/CLAUDE.md` mirrors the project-local operating contract for `.claude`
  scoped workflows.
- `../CLAUDE.md` holds the project-local operating contract.
- `../AGENTS.md` holds the Codex-facing agent instructions.
- `settings.json` defines shared permissions, confirmations, and hooks.
- `rules/` holds path-scoped boundary rules.
- `agents/` holds isolated high-context roles.
- `commands/` holds user-invoked slash commands.
- `skills/` holds reusable procedures that would otherwise bloat `CLAUDE.md`.
- `workflows/` holds deterministic orchestration contracts and report emitters.
- `hooks/` holds lifecycle guardrails and shared helper code.
- `output-styles/` holds response overlays for recurring report shapes.
- `agent-memory/` holds persistent notes for named agents.
- `hygiene/` holds generated maintenance state.
- `worktrees/` reserves workspace space for git worktrees.
- `security-patterns.yaml` and `claude-security-guidance.md` support security review and hook-side scanning.

## Read Order

1. `../CLAUDE.md`
2. `../AGENTS.md`
3. `settings.json`
4. `rules/README.md`
5. `agents/README.md`
6. `skills/README.md`
7. `commands/README.md`
8. `workflows/README.md`
9. `hooks/README.md`
10. `output-styles/README.md`
