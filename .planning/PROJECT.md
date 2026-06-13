# Desktop AI Client

## What This Is

Desktop AI Client is a desktop application scaffold for a local-history AI assistant built on Tauri, with a future Svelte frontend and a Rust backend. The architecture is intentionally least-privilege: the app keeps conversation history, attachments, and operational state local while brokering model access through explicit provider routing and hardened preview surfaces.

## Core Value

Keep local history, files, and agent state private while safely routing AI inference, streaming, and artifacts through explicit backend boundaries.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Build a working desktop shell with clear frontend/backend boundaries
- [ ] Route prompts through deterministic provider selection and streaming transport
- [ ] Persist conversation history locally with searchable SQLite storage
- [ ] Enforce privacy controls around secrets, file access, and telemetry
- [ ] Sandbox generated artifact previews so they cannot compromise the host app
- [ ] Gate release readiness with reviewed command inventory and adversarial evidence

### Out of Scope

- Raw frontend access to provider secrets or secret storage - secrets stay backend-owned
- Arbitrary SQL execution from the frontend - persistence must use typed backend commands
- Unrestricted remote assets in privileged windows - release builds must keep app surfaces controlled
- Direct raw-path file reads from JavaScript - attachment intake must be tokenized or Rust-owned

## Context

This repository already contains a docs-first architecture and an intentionally small Rust/Tauri module tree, but the executable product is still mostly placeholder code. The most important design signals live in `docs/architecture.md`, `docs/privacy-boundaries.md`, `docs/provider-routing.md`, `docs/threat-model.md`, and the adversarial architecture spec in `docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md`.

The codebase map shows the current backend layout and confirms that the application has not yet been wired into a buildable product. That means the initial planning work must treat the docs as the source of truth while preserving the existing boundary structure.

## Constraints

- **Architecture**: Tauri v2 backend with a future Svelte 5 frontend - the repo is organized around that split
- **Privacy**: secrets, file contents, and telemetry must stay within explicit boundaries - the architecture treats leakage as a release blocker
- **Storage**: local SQLite/FTS5 history with migrations, retention, and backup behavior - search and persistence must remain recoverable
- **Security**: provider routing, command policy, and artifact previews must remain isolated - hostile renderer behavior is a named threat
- **Release**: command inventory, capability selection, and adversarial fixtures are part of done - compilation alone is not enough

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Structure the roadmap as vertical MVP slices | Delivers end-to-end value early and keeps each phase testable | Pending |
| Treat cloud inference as a routed backend capability rather than a frontend concern | Preserves privacy and keeps provider logic behind the Rust boundary | Pending |
| Keep secrets and raw file access backend-owned | Prevents the frontend from becoming a trust boundary escape hatch | Pending |

## Evolution

This document will evolve as the implementation moves from scaffold to working product. Update it when requirements are validated, constraints change, or the repo’s architecture drifts beyond the current docs.

---
*Last updated: 2026-06-13 after initialization*
