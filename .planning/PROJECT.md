# Project: Desktop AI Client

## Core Value

Build a Tauri desktop client that keeps privacy-sensitive behavior, storage, command policy, and provider routing in the Rust backend while the Svelte renderer stays thin and communicates only through typed IPC. The product direction is memory-first: preserve useful lessons, minimize context bloat, and keep the single-agent loop stable before expanding into more complex coordination.

## Target Runtime

Tauri desktop

## Success Metric

At least 85% of well-scoped development tasks produce a review-ready pull request that satisfies every acceptance criterion and passes all required automated verification without manual code repair.

## Operating Principles

- Renderer/backend communication stays on typed Tauri IPC commands only.
- Privacy-sensitive concerns remain backend-owned: provider credentials, file paths, SQLite storage, command policy, and telemetry never become renderer-owned state.
- Strict privacy and command policy fail closed instead of silently downgrading behavior.
- The evidence-gated memory engine stays in shadow mode until a later phase explicitly decides it can influence live behavior.
- File intake is metadata-first and Rust-owned before any content is read.
- Multi-agent expansion waits until the single-agent loop is stable and measurable.
- Small, local changes are preferred over broad rewrites.

## Implementation Order

The roadmap follows the repo's stated build order:

1. Single-agent loop
2. Persistent trace storage
3. Retrieval
4. Verification and promotion
5. Consolidation
6. Observability
7. Guardrails
8. Multi-agent split

## Key Decisions

| Decision | Basis |
|----------|-------|
| Tauri desktop shell with a Svelte 5 renderer and Rust backend | `docs/architecture.md` |
| Typed IPC is the only renderer/backend boundary | `docs/architecture.md` |
| Backend owns provider credentials, storage, command policy, and telemetry | `docs/architecture.md`, `docs/privacy-boundaries.md` |
| The evidence-gated memory engine begins in shadow mode only | `docs/architecture.md`, `docs/memory-loop.md` |
| Strict privacy and command inventory checks fail closed | `docs/provider-routing.md`, `docs/command-inventory.md` |
| Multi-agent expansion is deferred until the single-agent loop is stable | `docs/implementation-plan.md`, `docs/agent-context.md` |

## Notes

- No ADRs or PRDs were present in the ingest set.
- No v1 requirements were extracted.
- The project shape is bootstrapped from synthesized docs context and constraints.
