# storage/AGENTS.md

This subtree owns persistence and retention.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/privacy-boundaries.md`
3. `../../../docs/threat-model.md`
4. `../AGENTS.md`

## Rules

- Keep migrations explicit.
- Keep backups and retention policies separate.
- Do not mix storage concerns with UI or provider logic.
- Prefer recoverable, auditable persistence paths.

