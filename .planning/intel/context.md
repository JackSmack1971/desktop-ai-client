# Context

## Repo direction and vocabulary
source: `docs/agent-context.md`

- The repository is being set up for coding agents that work on a long-running, memory-aware system.
- The current direction is memory-first: working memory, episodic memory, procedural memory, and caution memory.
- Agents should optimize for fewer repeated mistakes, shorter recovery time, better reuse of workflows, less context bloat, and stable behavior across sessions.
- Agents should avoid treating raw chat as truth, storing everything durably, over-retrieving stale memory, widening into multiple roles too early, or changing memory format without a migration plan.

## Docs read order
source: `docs/README.md`

- The repo's agent-facing docs establish a read order that starts with the root `AGENTS.md`, then the context, architecture, memory, prompt blueprint, design blueprint, templates, implementation plan, and multi-agent rules.
- These docs exist to keep agents aligned, prevent drift, capture the memory-loop design, define prompt rules, and separate single-agent from multi-agent concerns.

## Prompt templates overview
source: `docs/prompt-templates/README.md`

- The reusable prompt drafts are ordered coding, design, memory, then coordination.
- The templates are meant to be adapted for the task, but their structure and guardrails should stay intact.

## Coding prompt template
source: `docs/prompt-templates/coding-agent.md`

- Coding prompts should keep scope narrow, preserve existing invariants, and use the smallest change that solves the task.
- The workflow is inspect, implement, verify, and report.
- If a new contract is discovered, the docs should be updated.

## Design prompt template
source: `docs/prompt-templates/design-agent.md`

- Design prompts should define the goal, target user, main action, and visible states.
- They should choose a clear visual direction, avoid generic AI dashboard patterns, keep hierarchy obvious, and keep motion meaningful.
- Accessibility and readability are explicit constraints.

## Memory prompt template
source: `docs/prompt-templates/memory-agent.md`

- Memory prompts should summarize, promote, or consolidate memory from run data while keeping memory narrow and curated.
- Raw traces stay separate from promoted memory, and promotion requires verification.
- Duplicates and weak claims should be rejected; reusable lessons are preferred over one-off details.

## Coordination prompt template
source: `docs/prompt-templates/coordination-agent.md`

- Coordination prompts should route work between agents without losing context.
- Each task should have one owner, structured handoffs, a clear acceptance criterion, and a rollback path.

## Memory loop
source: `docs/memory-loop.md`

- The memory loop exists to decide what should be remembered, forgotten, and reused.
- Candidate memory records carry type, summary, source run, tags, confidence, utility, recency, verification state, and expiry.
- Retrieval should stay small and selective, and promotion should only happen when the trace supports it and the candidate is not a duplicate.
- Phase 1 implements the storage shape in shadow mode only; the live chat path does not consume it yet.

## Multi-agent rules
source: `docs/multi-agent.md`

- The repo should not split into multiple agents until the single-agent loop is stable and measurable.
- If multi-agent work becomes necessary, the suggested roles are planner, executor, memory writer, judge, and coordinator.
- Shared memory should stay local by default and only validated reusable lessons should be shared.

## Design blueprint
source: `docs/design-blueprint.md`

- Design prompts should make intentional layouts, preserve the repo's visual direction, and avoid generic AI UI patterns.
- Each prompt should target one outcome, define hierarchy intentionally, and use motion only when it communicates state.
- The interface should explain what the system is doing, what memory it used, why a decision was made, and what needs review.

## Implementation plan
source: `docs/implementation-plan.md`

- The implementation path starts with a single-agent core, then memory schema, run loop, retrieval, promotion and verification, consolidation and observability, and only then multi-agent expansion.
- The build order is single-agent loop, persistent trace storage, retrieval, verification and promotion, consolidation, observability, guardrails, and finally multi-agent split.

## Privacy boundaries
source: `docs/privacy-boundaries.md`

- Privacy work centers on secrets handling, file content visibility, retention, telemetry redaction, local storage scope, and strict privacy mode.
- Attachment intake is metadata-only before content read, and opaque file tokens are revoked after a successful read.
- Backend-owned system prompt content is never accepted from IPC, and the audit-safe receipt is intended to be safe for logging or display.

## Command inventory
source: `docs/command-inventory.md`

- The reviewed command inventory is the release gate source of truth for every custom Tauri command exposed by the app.
- Release verification compares the reviewed inventory, registered commands, build-time allowlist, permission grants, and the selected release capability.
- The verifier fails closed if any command is missing, extra, debug-only, or mismatched across those sources.

## Release evidence
source: `docs/release-evidence.md`

- Release evidence is a source-controlled summary of implemented-path verification and the fixture families behind it.
- The bundle records security checks, streaming tests, database/storage evidence, provider-routed evidence, command-inventory verification, artifact sandbox and accessibility evidence, and adversarial fixture coverage.
- Missing fixture families are represented as deferred rather than fabricated proof.

## Threat model
source: `docs/threat-model.md`

- The current threat model centers on provider routing abuse, secret exposure, hostile renderer behavior, unsafe command execution, and file access boundary violations.
- The documented mitigations cover renderer role injection, unconstrained model/parameter overrides, privacy downgrades, unbounded attachment ingestion, and token-map growth.
- Memory retrieval is explicitly deferred by design until a later phase wires it into live prompts with an explicit mitigation.
