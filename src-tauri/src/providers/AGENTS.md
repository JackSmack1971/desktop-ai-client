# providers/AGENTS.md

This subtree owns provider integrations and routing.

## Read first

Before editing code here, read:

1. `../../../AGENTS.md`
2. `../../../docs/provider-routing.md`
3. `../../../docs/privacy-boundaries.md`
4. `../AGENTS.md`

## Rules

- Keep provider capability detection explicit.
- Keep routing logic deterministic and testable.
- Do not store secrets in provider modules.
- Do not let provider choice leak into unrelated layers.
