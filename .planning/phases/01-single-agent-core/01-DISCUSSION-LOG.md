# Phase 1: Single-Agent Core - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-06-20
**Phase:** 1-Single-Agent Core
**Areas discussed:** Task/Run Schema, Startup Recovery, Success Metric and Workflow Boundary

---

## Task/Run Schema

| Option | Description | Selected |
|--------|-------------|----------|
| Use the existing conversation/turn/attempt model | Treat the current backend identifiers as the Phase 1 contract. | ✓ |
| Add a separate run layer | Group one or more turns under a new run abstraction. | |
| Keep the current model now, reserve a future run layer | Delay the abstraction but explicitly note it. | |

**User's choice:** Use the existing conversation/turn/attempt model as the Phase 1 contract.
**Notes:** No separate run layer in Phase 1.

---

## Startup Recovery

| Option | Description | Selected |
|--------|-------------|----------|
| Restore shell state and recover orphaned attempts only | Keep recovery limited to backend crash cleanup. | ✓ |
| Auto-reopen the last active conversation | Resume the prior conversation automatically on startup. | |
| Restore shell state, recover orphaned attempts, and leave reopening to the user | Recovery stays limited, conversation re-entry is explicit. | |

**User's choice:** Restore shell state and recover orphaned attempts, but conversation reopening stays user-driven.
**Notes:** No automatic reopen on startup.

---

## Success Metric and Workflow Boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Record both in project memory and phase context now | Duplicate the metric and boundary in both places. | |
| Record only the workflow boundary now | Keep the success metric implicit for planning. | |
| Record the success metric in project memory and the workflow boundary in phase context | Keep the metric in project-level memory and the boundary in this phase artifact. | ✓ |

**User's choice:** Record the success metric in project memory and the workflow boundary in phase context.
**Notes:** The metric stays in `.planning/PROJECT.md`; Phase 1 context captures the boundary.

---

## the agent's Discretion

None.

## Deferred Ideas

None.
