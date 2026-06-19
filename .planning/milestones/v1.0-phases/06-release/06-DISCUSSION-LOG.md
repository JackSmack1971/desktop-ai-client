# Phase 6: Release - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-06-15
**Phase:** 06-release
**Areas discussed:** command inventory enforcement, capability selection, release evidence scope

---

## Command Inventory Enforcement

| Option                    | Description                                                                                                       | Selected |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------- | -------- |
| Markdown placeholder only | Keep the current docs stub and describe the inventory informally.                                                 |          |
| TOML inventory + verifier | Use `security/command-inventory.toml` plus a verifier that checks registration, capabilities, and inventory sync. | ✓        |
| Let the agent decide      | Defer the inventory shape to planning.                                                                            |          |

**User's choice:** Strict command inventory enforcement for all registered custom commands, with a reviewed inventory backed by `security/command-inventory.toml` and verifier enforcement.
**Notes:** The gate should fail if any registered custom command is absent from the reviewed inventory.

---

## Capability Selection

| Option                     | Description                                                                                                   | Selected |
| -------------------------- | ------------------------------------------------------------------------------------------------------------- | -------- |
| Single release capability  | Keep `main.json` as the only capability file and treat directory presence as enough.                          |          |
| Explicit release/dev split | Keep release capabilities intentionally selected and separate dev-only capability files from the release set. | ✓        |
| Let the agent decide       | Defer capability layout to planning.                                                                          |          |

**User's choice:** Introduce explicit release/dev capability separation.
**Notes:** `src-tauri/capabilities/main.json` remains the release-selected baseline unless future release capabilities are added intentionally.

---

## Release Evidence Scope

| Option                       | Description                                                                                  | Selected |
| ---------------------------- | -------------------------------------------------------------------------------------------- | -------- |
| Narrow pass only             | Require only the smallest release gate evidence needed for currently implemented paths.      |          |
| Full hardening bundle        | Require every evidence file from the hardening spec, regardless of implementation status.    |          |
| First-pass structured bundle | Preserve the full hardening-spec structure, but require evidence only for implemented paths. | ✓        |

**User's choice:** Produce a first-pass release evidence bundle covering implemented paths only while preserving the full hardening-spec evidence structure.
**Notes:** The evidence bundle should stay aligned with the hardening spec's categories and fixture families, but should not force fabricated proof for deferred features.

---

## the agent's Discretion

- Exact verifier implementation details
- Evidence collection scripts and packaging mechanics

## Deferred Ideas

- None
