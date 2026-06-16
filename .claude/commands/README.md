# Commands

Commands are user-invoked entrypoints. Keep them narrow and report-driven.

## Canonical Surface

- `/create:pr` implements one issue as a bounded change.
- `/review:pr` reviews a diff or PR for merge readiness.
- `/audit:repo` runs a broad desktop-client audit.
- `/audit:privacy` runs a focused privacy and hostile-surface audit.
- `/release:readiness` checks release evidence and packaging readiness.

## Composition Order

1. Commands should invoke a workflow contract.
2. Workflows should compose skills and agents.
3. Agents should keep their own output contracts narrow.

