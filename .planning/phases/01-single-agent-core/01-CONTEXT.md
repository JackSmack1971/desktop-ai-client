# Phase 1: Single-Agent Core - Context

**Gathered:** 2026-06-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 delivers the first durable single-agent workflow contract: the existing conversation/turn/attempt model is the phase boundary, startup recovery restores shell state and recovers orphaned attempts, and the project records the success metric in project memory while this phase context captures the workflow boundary.

</domain>

<decisions>
## Implementation Decisions

### Task/Run Schema
- **D-01:** Use the existing `conversation` / `turn` / `attempt` model as the Phase 1 contract.
- **D-02:** Do not add a separate `run` layer in Phase 1.

### Startup Recovery
- **D-03:** On startup, restore shell state and recover orphaned attempts only.
- **D-04:** Do not auto-reopen the last active conversation; reopening stays user-driven.

### Success Metric and Workflow Boundary
- **D-05:** Keep the success metric in `.planning/PROJECT.md` as project memory.
- **D-06:** Record the Phase 1 workflow boundary in this context file.

### the agent's Discretion
None - the discussed areas were decided explicitly.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level context
- `.planning/PROJECT.md` - project success metric, operating principles, and implementation order.
- `.planning/STATE.md` - current project status and phase position.
- `.planning/ROADMAP.md` - phase goals and phase boundaries.

### Architecture and storage contracts
- `docs/architecture.md` - Tauri shell/backend boundary, storage ownership, and conversation transaction protocol.
- `docs/memory-loop.md` - memory engine shape and shadow-mode constraints.
- `docs/implementation-plan.md` - milestone ordering and build order.
- `docs/agent-context.md` - memory-first agent direction and terminology.
- `docs/privacy-boundaries.md` - privacy boundary, attachment intake, and memory engine storage notes.
- `docs/command-inventory.md` - command inventory invariants and release gating.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src-tauri/src/storage/turns.rs` - already implements the conversation/turn/attempt contract, idempotency, retries, and orphan recovery.
- `src-tauri/src/ipc/chat.rs` - already wires chat send/cancel around the turn store and recovery semantics.
- `src/lib/stores/chat.ts` - existing frontend retry/hydration behavior already matches the turn model.
- `src/lib/stores/history.ts` - history hydration and active conversation tracking already exist.

### Established Patterns
- Backend-owned state stays in Rust and crosses IPC only through typed commands.
- Startup recovery happens in `src-tauri/src/main.rs` before IPC commands can observe in-flight state.
- Existing chat semantics already treat `conversation_id`, `turn_id`, and `attempt_id` as the durable identifiers.

### Integration Points
- `src-tauri/src/main.rs` - startup wiring and orphaned attempt recovery.
- `src-tauri/src/storage/migrations.rs` - turn and attempt schema migrations.
- `src/lib/api/chat.ts` and `src/lib/stores/chat.ts` - frontend turn submission and retry flow.

</code_context>

<specifics>
## Specific Ideas

- No separate run abstraction in Phase 1; the existing turn model is the contract.
- Conversation reopening should remain a user action rather than an automatic startup behavior.

</specifics>

<deferred>
## Deferred Ideas

None - discussion stayed within phase scope.

</deferred>

---

*Phase: 01-Single-Agent Core*
*Context gathered: 2026-06-20*
