# Roadmap: Desktop AI Client

## Overview

This project is being bootstrapped from synthesized docs context rather than PRD-backed v1 requirements, so the roadmap follows the repo's documented memory-first implementation order. The sequence starts with a stable single-agent core, then adds the memory schema and shadow-mode pipeline, then run-loop tracing, then bounded retrieval and promotion, then consolidation and observability guardrails, and only then any multi-agent expansion.

## Phases

- [x] **Phase 1: Single-Agent Core** - Define the first workflow, success metric, task/run schemas, and durable startup recovery.
- [ ] **Phase 2: Memory Model** - Separate durable memory from raw traces and establish the memory record shape.
- [ ] **Phase 3: Run Loop** - Record runs end-to-end, including trace events, summaries, and candidate memory emission.
- [ ] **Phase 4: Retrieval, Promotion, and Verification** - Make bounded retrieval and trace-backed promotion decisions testable.
- [ ] **Phase 5: Consolidation, Observability, and Guardrails** - Dedupe, expire, observe, and harden the memory system before it widens.
- [ ] **Phase 6: Multi-Agent Expansion** - Split roles only after the single-agent loop is stable and measured.

## Phase Details

### Phase 1: Single-Agent Core
**Status**: Complete
**Goal**: Users can define one task/run workflow, persist it durably, and recover it on startup without losing the active context.
**Depends on**: Nothing (first phase)
**Requirements**: None
**Success Criteria** (what must be TRUE):
  1. A new run can be created and loaded back after restart with its identity intact.
  2. The project has explicit task/run schemas that distinguish the active workflow from historical traces.
  3. Startup retrieval restores the last relevant working context before any new work begins.
  4. The success metric and workflow boundary are recorded as part of project memory instead of staying implicit.
**Plans**: 1

### Phase 2: Memory Model
**Goal**: The system can store candidate memories separately from raw traces without letting live chat depend on them.
**Depends on**: Phase 1
**Requirements**: None
**Success Criteria** (what must be TRUE):
  1. Memory records are stored in tables or stores separate from raw conversation traces.
  2. Each memory candidate carries type, summary, source, tags, confidence, utility, recency, verification state, and expiry.
  3. The memory pipeline remains shadow-mode only and does not change live provider routing or chat behavior.
  4. A human can inspect stored memory metadata without exposing backend-owned secrets or raw file paths.
**Plans**: TBD

### Phase 3: Run Loop
**Goal**: Every run is captured as a traceable execution record with summary and candidate memory output.
**Depends on**: Phase 2
**Requirements**: None
**Success Criteria** (what must be TRUE):
  1. A completed run records the events that happened during execution in order.
  2. Tool and turn events are appended to the run trace rather than reconstructed later from chat history.
  3. Each run produces a compact summary that can be replayed or reviewed later.
  4. Finished runs can emit candidate memories for later evaluation without promoting them automatically.
**Plans**: TBD

### Phase 4: Retrieval, Promotion, and Verification
**Goal**: Retrieval and promotion behave as bounded, explainable gates instead of implicit memory drift.
**Depends on**: Phase 3
**Requirements**: None
**Success Criteria** (what must be TRUE):
  1. Retrieval returns only a small bounded set of candidate memories.
  2. Retrieval prefers relevant, recent, confident, and useful items while excluding expired ones.
  3. Promotion only succeeds when the trace supports the candidate and duplicate or contradiction checks pass.
  4. A human can see why a candidate was promoted, deferred, or rejected.
**Plans**: TBD

### Phase 5: Consolidation, Observability, and Guardrails
**Goal**: The memory system becomes maintainable and observable without weakening the privacy boundary.
**Depends on**: Phase 4
**Requirements**: None
**Success Criteria** (what must be TRUE):
  1. Duplicate and overlapping memories can be deduped, merged, or expired instead of accumulating forever.
  2. Retrieval and promotion activity is observable through logs or metrics that do not expose sensitive content.
  3. Regressions in memory pollution or stale retrieval are detectable through automated tests.
  4. The command, provider, and privacy boundaries remain deny-by-inventory and fail closed.
**Plans**: TBD

### Phase 6: Multi-Agent Expansion
**Goal**: Multi-agent roles can be introduced only after the single-agent loop is stable and trustworthy.
**Depends on**: Phase 5
**Requirements**: None
**Success Criteria** (what must be TRUE):
  1. The system can split into planner, executor, memory writer, and judge roles without losing the single-agent baseline.
  2. Shared procedures are reused only after they are verified as stable lessons.
  3. Inter-agent communication stays structured and local memory remains the default.
  4. A coordinator is introduced only after the single-agent loop is demonstrably stable.
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Single-Agent Core | 1/1 | Complete | 2026-06-20 |
| 2. Memory Model | TBD | Not started | - |
| 3. Run Loop | TBD | Not started | - |
| 4. Retrieval, Promotion, and Verification | TBD | Not started | - |
| 5. Consolidation, Observability, and Guardrails | TBD | Not started | - |
| 6. Multi-Agent Expansion | TBD | Not started | - |
