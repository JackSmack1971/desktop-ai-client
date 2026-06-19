# security/AGENTS.md

This subtree owns secrets, redaction, sandboxing, and command policy.

## Read first

Before editing code here, read:

1. `../../../AGENTS.md`
2. `../../../docs/threat-model.md`
3. `../../../docs/privacy-boundaries.md`
4. `../AGENTS.md`

## Rules

- Treat secrets handling as a hard boundary.
- Redact sensitive data before it reaches logs or telemetry.
- Keep command policy separate from provider routing.
- Keep artifact sandboxing explicit and narrow.
