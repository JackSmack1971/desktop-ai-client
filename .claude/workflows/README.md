# Workflows

Workflows keep orchestration state out of model context.

## Workflow Stack

- `issue-to-pr.js`: one issue to one PR-sized implementation flow
- `review-readiness.js`: local diff or PR review flow
- `repo-audit.js`: broad audit flow for the desktop client
- `release-readiness.js`: packaging and release gate flow
- `pr-rescue-factory.js`: repair a broken PR or finish an interrupted implementation
- `document-taxonomy-corpus.js`: classify and normalize repository documents
- `audit-claude-context-compression.js`: audit `.claude/` context size and duplication

## Shared Expectations

- declare input shape, phase order, risk surfaces, verification checks, and output sections
- keep long-running orchestration logic in JavaScript instead of bloating model context
- prefer report emitters and contract JSON over free-form prose when the workflow is machine-driven

