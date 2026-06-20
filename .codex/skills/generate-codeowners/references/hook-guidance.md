# Session-scoped hook guidance

The package includes `references/hook-settings.json` for deterministic command governance. It is intentionally session-scoped: do not merge its restrictive Write/Edit policy into normal project settings.

## Governed launch

Install the skill at `.claude/skills/generate-codeowners/`, then launch Claude Code from the repository root with:

```bash
claude --settings .claude/skills/generate-codeowners/references/hook-settings.json
```

Invoke `/generate-codeowners` inside that session.

Behavior:

- `PreToolUse` rejects destructive cleanup, Git history rewriting, permission/ruleset mutation, and Write/Edit targets outside `.github/CODEOWNERS` plus the Git metadata state directory.
- `Stop` permits audit mode when no generated `.github/CODEOWNERS` exists. When the file exists, it blocks completion on validator errors.
- Hooks tighten behavior but do not grant permissions; existing permission rules still apply.
- Run `/hooks` to confirm both handlers are registered.

The profile assumes project-scoped installation because its commands resolve scripts from `.claude/skills/generate-codeowners/`. Personal installations should run the workflow without this profile or copy the skill into the project first.
