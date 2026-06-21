# Commands

Commands are user-invoked entrypoints. Keep them narrow and report-driven.

## Canonical Surface

- `create/pr.md` implements one issue as a bounded change.
- `review/pr.md` reviews a diff or PR for merge readiness.
- `audit/repo.md` runs a broad desktop-client audit.
- `audit/privacy.md` runs a focused privacy and hostile-surface audit.
- `audit/context-compression.md` audits `.claude/` context size and duplication.
- `release/readiness.md` checks release evidence and packaging readiness.
- `worktree.md` and `worktrees.md` are compatibility aliases for stale worktree cleanup.

## Composition Order

1. Commands should invoke a workflow contract.
2. Workflows should compose skills and agents.
3. Agents should keep their own output contracts narrow.
