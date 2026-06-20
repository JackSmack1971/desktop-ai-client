---
phase: 01-single-agent-core
plan: '01'
type: execute
wave: 1
depends_on: []
files_modified:
  - src-tauri/src/storage/turns.rs
  - src-tauri/src/storage/sqlite.rs
  - src-tauri/src/storage/migrations.rs
  - src-tauri/src/main.rs
  - src-tauri/src/app_state.rs
  - src-tauri/src/ipc/chat.rs
  - src-tauri/tests/app_shell.rs
  - src/lib/stores/chat.ts
  - src/lib/stores/history.ts
  - src/routes/+layout.svelte
  - .planning/PROJECT.md
  - .planning/STATE.md
  - .planning/ROADMAP.md
autonomous: true
requirements: []
must_haves:
  truths:
    - The active workflow is the conversation/turn/attempt contract, not a separate run layer.
    - Backend startup completes SQLite open, migrations, and orphan recovery before any renderer-facing command can observe readiness.
    - Shell state is limited to backend-owned active surface plus hydration state; active conversation ownership stays out of shell state.
    - SQLite unavailability, corrupted migration state, and partial orphan recovery fail closed instead of exposing stale readiness.
    - Restart restores shell state but does not auto-reopen the last active conversation.
    - The success metric and phase boundary are recorded in project memory instead of staying implicit.
  artifacts:
    - src-tauri/src/storage/turns.rs
    - src-tauri/src/storage/sqlite.rs
    - src-tauri/src/storage/migrations.rs
    - src-tauri/src/main.rs
    - src-tauri/src/app_state.rs
    - src-tauri/src/ipc/chat.rs
    - src-tauri/tests/app_shell.rs
    - src/lib/stores/chat.ts
    - src/lib/stores/history.ts
    - .planning/PROJECT.md
    - .planning/STATE.md
    - .planning/ROADMAP.md
  key_links:
    - Turn identity is owned in Rust storage and only surfaced to the renderer through typed IPC.
    - Startup recovery completes in the Tauri setup hook before `get_active_surface` or any other renderer-facing command can observe shell readiness.
    - `src/routes/+layout.svelte` only consumes backend readiness/surface hydration; it must not reclaim active-conversation ownership from backend state.
    - The project success metric belongs in `.planning/PROJECT.md` so later phases inherit it, and `.planning/STATE.md` / `.planning/ROADMAP.md` are updated last when the phase is actually complete.
---

<objective>
Establish the first single-agent core plan: keep the conversation/turn/attempt model as the phase boundary, make startup recovery authoritative and fail closed, and record the project success metric in project memory so later phases execute against a stable contract.
</objective>

<execution_context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/01-single-agent-core/01-CONTEXT.md
@docs/architecture.md
@docs/memory-loop.md
@docs/implementation-plan.md
@docs/agent-context.md
@docs/privacy-boundaries.md
@src-tauri/AGENTS.md
@src-tauri/src/AGENTS.md
@src-tauri/src/storage/AGENTS.md
@src/lib/api/AGENTS.md
@src/lib/stores/AGENTS.md
</execution_context>

<context>
## Phase Boundary

Phase 1 delivers the durable single-agent workflow contract: the existing conversation/turn/attempt model is the phase boundary, startup recovery restores shell state and orphaned attempts, and the project records the success metric in project memory while this phase context captures the workflow boundary.

## Implementation Decisions

### Task/Run Schema
- Use the existing `conversation` / `turn` / `attempt` model as the Phase 1 contract.
- Do not add a separate `run` layer in Phase 1.

### Startup Recovery
- On startup, restore shell state and recover orphaned attempts only.
- Do not auto-reopen the last active conversation; reopening stays user-driven.

### Success Metric and Workflow Boundary
- Keep the success metric in `.planning/PROJECT.md` as project memory.
- Record the Phase 1 workflow boundary in this context file.

### the agent's Discretion
- Keep UI polish and future memory promotion work out of Phase 1.
</context>

<tasks>
<task type="auto">
  <name>Lock the schema boundary</name>
  <files>src-tauri/src/storage/turns.rs, src-tauri/src/storage/sqlite.rs, src-tauri/src/storage/migrations.rs, src-tauri/src/ipc/chat.rs</files>
  <action>Keep the active workflow on conversation/turn/attempt identities, make the storage API expose that contract cleanly, and tighten tests around idempotent retries and orphan recovery so the phase boundary cannot drift into a separate run abstraction or leak active-conversation state into shell state.</action>
  <verify>Run `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` and confirm the turn-store and recovery cases still pass.</verify>
  <acceptance_criteria>
    - No separate run layer is introduced in Phase 1.
    - A conversation can be resumed through an existing turn identity without duplicating the user message.
    - Retry and recovery paths remain idempotent.
  </acceptance_criteria>
</task>

<task type="auto">
  <name>Make startup recovery authoritative</name>
  <files>src-tauri/src/main.rs, src-tauri/src/app_state.rs, src-tauri/src/storage/turns.rs, src-tauri/src/ipc/chat.rs, src/routes/+layout.svelte, src-tauri/tests/app_shell.rs, src/lib/stores/chat.ts, src/lib/stores/history.ts</files>
  <action>Define the startup sequencing contract so the Tauri setup hook finishes SQLite open, migrations, and orphan recovery before any renderer-facing command or readiness signal can proceed. Keep `ShellState` bounded to the backend-owned active surface plus hydration flag only, treat `src/routes/+layout.svelte` as a consumer of backend readiness rather than an owner of active-context state, and leave active conversation reopening user-driven through the history/chat flow. Make the startup path fail closed when SQLite is unavailable, migrations are corrupted, or orphan recovery is only partially successful, and add a restart-style regression in `src-tauri/tests/app_shell.rs` that proves restart hydration works while the last active conversation is not auto-reopened.</action>
  <verify>Run `cargo test --manifest-path src-tauri/Cargo.toml --test app_shell` and `npm run check` after the wiring is updated.</verify>
  <acceptance_criteria>
    - Backend startup completes recovery before any renderer-facing command can observe readiness.
    - `ShellState` contains only the backend-owned active surface and hydration state.
    - SQLite unavailability, corrupted migration state, and partial orphan recovery do not produce a ready shell state.
    - The last active conversation is not auto-reopened on restart.
  </acceptance_criteria>
</task>

<task type="auto">
  <name>Record the phase memory</name>
  <files>.planning/PROJECT.md, .planning/STATE.md, .planning/ROADMAP.md, .planning/phases/01-single-agent-core/01-CONTEXT.md</files>
  <action>After implementation and verification are complete, update `.planning/PROJECT.md` with the final success metric and Phase 1 boundary first, then update `.planning/STATE.md` and `.planning/ROADMAP.md` last so the phase is marked complete without altering later phases or leaving the completion markers out of sync.</action>
  <verify>Run `git diff --check` and confirm `.planning/PROJECT.md` reflects the final contract while `.planning/STATE.md` and `.planning/ROADMAP.md` show Phase 1 completion markers in the expected order.</verify>
  <acceptance_criteria>
    - The success metric is visible in project memory.
    - The phase boundary is explicit in the context artifact.
    - STATE and ROADMAP reflect completion only after the implementation artifacts land.
  </acceptance_criteria>
</task>
</tasks>

<threat_model>
## Trust Boundaries

- renderer -> IPC: frontend calls cross into backend commands through typed IPC only.
- backend -> SQLite: turn storage owns identity, retry, and recovery; the renderer never issues raw SQL.
- app handle -> filesystem: startup recovery uses the app-owned data directory and creates backend state before new work begins.

## Key Constraints

- Do not add a separate run abstraction in Phase 1.
- Do not move active workflow state into browser storage.
- Keep startup recovery fail-closed if persistence is unavailable.
- Do not surface renderer readiness until backend startup recovery has completed.
</threat_model>

<verification>
Phase-level checks after the tasks are implemented:

1. `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` exits 0.
2. `npm run check` exits 0.
3. `git diff --check` exits 0.
4. A restart restores the last relevant active context without reopening history automatically.
</verification>

<success_criteria>
- The single-agent core contract is explicit and durable.
- Startup recovery happens before new work begins.
- The project success metric is recorded in project memory.
- Phase 1 does not introduce a separate run layer.
</success_criteria>
