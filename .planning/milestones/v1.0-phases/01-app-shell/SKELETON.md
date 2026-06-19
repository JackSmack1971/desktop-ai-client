# Walking Skeleton - Desktop AI Client

**Phase:** 1
**Generated:** 2026-06-13

## Capability Proven End-to-End

A user can launch the app, land in a backend-owned workspace shell, switch between chat, history, settings, and artifact surfaces, and have the last selected surface restored on relaunch.

## Architectural Decisions

| Decision          | Choice                                                                                   | Rationale                                                                                                |
| ----------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Framework         | Svelte 5 with runes and a SvelteKit shell                                                | Matches the documented frontend direction and keeps the shell reactive without widening the UI boundary. |
| Data layer        | SQLite-backed shell preference store                                                     | Proves a real backend read/write path while keeping state backend-owned.                                 |
| Desktop framework | Tauri v2                                                                                 | Preserves explicit IPC, command boundaries, and a small desktop runtime surface.                         |
| Auth              | None yet                                                                                 | Phase 1 is shell bootstrap; provider auth and secret handling belong to later phases.                    |
| Deployment target | Local dev desktop run via `npm run dev`                                                  | Exercises the real desktop runtime instead of a mocked browser-only path.                                |
| Directory layout  | `src/lib/components`, `src/lib/stores`, `src-tauri/src/{ipc,storage,security,providers}` | Keeps ownership explicit and separates shell UI from backend policy modules.                             |

## Stack Touched in Phase 1

- [ ] Project scaffold (framework, build, lint, test runner)
- [ ] Routing - at least one real route
- [ ] Database - at least one real read AND one real write
- [ ] UI - at least one interactive element wired to the API
- [ ] Deployment - running on dev environment OR documented local full-stack run command

## Out of Scope (Deferred to Later Slices)

- Conversation history persistence and search
- Prompt routing and provider selection
- Privacy hardening beyond the shell boundary
- Artifact sandboxing and preview execution
- Release evidence and reviewed command inventory

## Subsequent Slice Plan

Each later phase adds one vertical slice on top of this skeleton without altering its architectural decisions:

- Phase 2: deterministic provider routing and streaming transport
- Phase 3: local conversation history with search and retention
- Phase 4: privacy controls for secrets, file access, and telemetry
- Phase 5: sandboxed artifact previews and recovery controls
- Phase 6: release readiness with reviewed command inventory and adversarial evidence
