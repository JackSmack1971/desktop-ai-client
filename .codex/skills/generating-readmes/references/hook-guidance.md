# Optional Claude Code Hook Guidance for README Generation

Use these rules when a project wants the README workflow enforced by Claude Code lifecycle hooks. Keep hook implementation outside this skill package in the project or user Claude Code settings. The skill remains usable without hooks.

## PreToolUse policy

Block or require confirmation for commands that are not needed for README generation:

- Destructive filesystem actions: recursive deletion, force removal, disk formatting, chmod/chown across broad paths.
- Secret-bearing reads: `.env`, `.npmrc`, private keys, credential stores, cloud credential files, SSH keys.
- Network and deployment actions: package publishing, Docker push, git push, cloud deploy, database migration, remote curl scripts.
- Package installation or dependency updates unless the user explicitly requested live verification.
- Writes outside the repository root.

Allow read-only repository discovery and safe bundled scripts:

- `git rev-parse`, `git status --short`, `git remote -v` when needed.
- `python ${CLAUDE_SKILL_DIR}/scripts/scan_repo.py --root . --format markdown`.
- `python ${CLAUDE_SKILL_DIR}/scripts/readme_quality_check.py --root . --readme README.md --format markdown`.
- Directory listing, grep, file reads, and README writes under the repository root.

## PostToolUse policy

After repository scans, summarize only the inventory sections needed for the README. After README writes, run the quality checker and surface the score, missing sections, broken local links, and unresolved `[INFERRED]` or `[TBD]` items.

## Stop / TaskCompleted policy

Do not allow a completion claim unless one of these is true:

- `--audit-only` was used and the response includes current score, gaps, and proposed changes.
- `--no-write` was used and the response includes a complete proposed README.
- A README file was written, the quality checker ran, and the final response reports the score and remaining gaps.

## SubagentStop policy

Because this skill uses `context: fork`, the subagent should return a compact mailbox summary to the parent session:

- Files changed.
- Evidence sources read.
- Commands or scripts run.
- Quality score.
- Remaining `[INFERRED]`, `[TBD]`, or unverified claims.
- Any safety blocks encountered.
