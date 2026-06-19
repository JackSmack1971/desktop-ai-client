# ipc/AGENTS.md

This subtree owns the commands the frontend can call.

## Read first

Before editing code here, read:

1. `../../../AGENTS.md`
2. `../../../docs/command-inventory.md`
3. `../../../docs/privacy-boundaries.md`
4. `../AGENTS.md`

## Rules

- Each command should do one thing.
- Commands should validate input at the boundary.
- Commands should return structured results.
- Do not place provider-specific logic in IPC handlers.
- Do not let IPC bypass security or redaction rules.
