# Workflows

Workflows keep orchestration state out of model context.

## Workflow Stack

- `issue-to-pr.js`: one issue to one PR-sized implementation flow
- `review-readiness.js`: local diff or PR review flow
- `repo-audit.js`: broad audit flow for the desktop client
- `release-readiness.js`: packaging and release gate flow

## Contract Expectations

- Each workflow should declare its required input, phase order, risk surfaces, verification checks, and output sections.
- Commands should point at workflows for orchestration.
- Skills should provide the deeper domain procedures behind each workflow step.

