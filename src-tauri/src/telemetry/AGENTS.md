# telemetry/AGENTS.md

This subtree owns audit logging and release evidence.

## Read first

Before editing code here, read:

1. `../../../AGENTS.md`
2. `../../../docs/release-evidence.md`
3. `../../../docs/threat-model.md`
4. `../AGENTS.md`

## Rules

- Telemetry must not leak secrets or private file contents.
- Keep audit events structured and time-ordered.
- Release evidence should be reproducible.
- Do not mix observability with behavior policy.
