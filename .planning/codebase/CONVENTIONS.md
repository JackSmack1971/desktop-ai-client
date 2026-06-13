# Conventions

## Observed conventions

- Docs-first project setup
- Strong module ownership boundaries
- Security and privacy concerns separated from provider routing
- Small leaf modules instead of large mixed-purpose files
- Placeholder functions named after their module concern

## Agent-facing conventions

- Read the nearest `AGENTS.md` before editing a subtree
- Treat docs as the source of truth for boundaries and behavior contracts
- Keep command policy, provider routing, storage, and telemetry separate
- Redact sensitive data before it reaches logs or telemetry

## Code style status

- No formatter, linter, or language-specific config was present in the snapshot
- No real implementation style can be inferred yet from the placeholder functions

## Maintenance risk

Because the code is mostly scaffold, conventions are currently stronger in docs than in code. That is acceptable for initialization, but it means implementation work must preserve the document-defined boundaries or the architecture will drift quickly.
