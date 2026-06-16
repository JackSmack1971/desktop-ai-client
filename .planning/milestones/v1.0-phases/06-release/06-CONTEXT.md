# Phase 6: Release - Context

**Gathered:** 2026-06-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the project release-ready by enforcing a reviewed command inventory, explicitly separating release and dev capabilities, and producing release evidence that matches the hardening-spec structure while only gating on implemented paths. This phase implements the release gate, not new product features.

</domain>

<decisions>
## Implementation Decisions

### Command Inventory Enforcement

- **D-01:** Phase 6 uses strict deny-by-inventory enforcement for all registered custom commands. A command is release-blocking unless it is listed in the reviewed inventory and matched against Rust registration plus capability grants.
- **D-02:** The reviewed inventory should live as a source-controlled release artifact, with `security/command-inventory.toml` as the canonical shape from the hardening spec. The release gate must compare the inventory against `tauri::generate_handler![...]`, capability selection, and the compiled command surface.
- **D-03:** Release verification must cover the full registered custom-command set, not just a release subset. Any custom command that is registered but missing from the reviewed inventory is a release failure.

### Capability Selection

- **D-04:** Phase 6 introduces an explicit release/dev capability split. Capability inclusion for release must be intentional, not inferred from directory presence.
- **D-05:** `src-tauri/capabilities/main.json` remains the release-selected capability for the main window unless additional release capabilities are explicitly added later. Any dev-only capability files must be clearly separated and excluded from the release capability set.

### Release Evidence

- **D-06:** Phase 6 produces a first-pass release evidence bundle that preserves the hardening-spec file structure but only requires evidence for implemented paths at the time of release. Deferred or unimplemented paths remain represented by the structure, not by forcing fabricated evidence.
- **D-07:** The evidence bundle must still follow the hardening-spec categories: security checks, streaming tests, database/storage evidence, provider-routed evidence, command-inventory verification, and adversarial fixture coverage. The phase gate should fail if implemented-path evidence is missing.
- **D-08:** Exact fixture families called out by the hardening spec remain the target shape for the bundle, including SSE errors, FTS query abuse, `srcdoc` escaping, WAL recovery, and capability drift, but only where the corresponding implementation exists to exercise them.

### the agent's Discretion

- Exact verifier implementation details, evidence collection scripts, and packaging mechanics can be chosen during planning as long as they enforce the decisions above.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope and Requirements
- `.planning/ROADMAP.md` — Phase 6 goal and success criteria for release readiness
- `.planning/REQUIREMENTS.md` — REL-01 and REL-02 definitions

### Release Evidence and Inventory Authority
- `docs/release-evidence.md` — release evidence focus areas and the placeholder contract to replace
- `docs/command-inventory.md` — reviewed command inventory placeholder and required fields
- `docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md` — primary release hardening authority; command inventory verifier, explicit release capabilities, and release evidence bundle structure

### Boundary and Policy Context
- `docs/provider-routing.md` — provider-routing evidence scope and terminology
- `docs/privacy-boundaries.md` — privacy boundary definitions that release evidence must preserve
- `docs/threat-model.md` — hostile-renderer and command-execution risks that release verification must cover

### Prior Phase Decisions
- `.planning/phases/05-artifacts/05-CONTEXT.md` — artifact sandboxing, fail-closed preview behavior, and artifact-related evidence expectations
- `.planning/phases/04-privacy/04-CONTEXT.md` — command policy, redaction, and file-token constraints that feed release verification
- `.planning/phases/03-history/03-CONTEXT.md` — history persistence, retention, and deletion behavior that feed storage evidence
- `.planning/phases/02-routing/02-CONTEXT.md` — streaming, cancellation, and credential-handling decisions that feed release evidence

### Codebase and Architecture References
- `.planning/codebase/ARCHITECTURE.md` — Tauri command-surface invariant, capability model, and release boundary rules
- `.planning/codebase/CONCERNS.md` — current release readiness gaps and known missing verification paths
- `.planning/codebase/TESTING.md` — current test coverage gaps and evidence categories
- `src-tauri/src/main.rs` — current registered command set and the release surface to inventory
- `src-tauri/capabilities/main.json` — current main-window capability grant
- `src-tauri/src/ipc/mod.rs` — command-surface contract and inventory sync note
- `src-tauri/src/telemetry/release_evidence.rs` — scaffold placeholder for release evidence capture
- `src-tauri/src/telemetry/AGENTS.md` — telemetry subtree ownership and reproducibility rules

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src-tauri/src/main.rs` already centralizes `tauri::generate_handler![...]`, so the release verifier can compare one registration site against the inventory and capabilities.
- `src-tauri/capabilities/main.json` already expresses the main-window permission set, which can become the release-selected capability baseline.
- `src-tauri/src/ipc/mod.rs` documents the three-way sync expectation between registration, capabilities, and inventory.
- `src-tauri/src/telemetry/release_evidence.rs` is the natural backend hook for release evidence capture.
- `docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md` already defines the evidence bundle shape and the required adversarial fixture families.

### Established Patterns
- The project already treats deny-by-default capability files as a release boundary, so release/dev separation should extend that model rather than invent a new one.
- Command registration is already centralized in `main.rs`, which makes inventory verification mechanically feasible.
- The phase history shows that previous decisions are meant to be carried forward rather than re-litigated; release verification should build on Phase 2 through Phase 5 contracts.

### Integration Points
- `src-tauri/src/telemetry/release_evidence.rs` for release evidence generation and packaging
- `src-tauri/src/main.rs` for command inventory discovery
- `src-tauri/capabilities/main.json` and any future dev-only capability files for explicit release selection
- `docs/command-inventory.md` and `security/command-inventory.toml` for the reviewed inventory source of truth

</code_context>

<specifics>
## Specific Ideas

- Phase 6 should fail release if any registered custom command is absent from the reviewed inventory.
- Release/dev capability separation must be explicit, not implied by folder layout.
- The evidence bundle should preserve the hardening-spec structure even when only some evidence files are required for the current implementation.
- Evidence should cover implemented security, routing, storage, and adversarial fixture paths without inventing proof for deferred features.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 06-Release*
*Context gathered: 2026-06-15*
