# Memory Loop

## Goal

The memory loop exists to help the agent improve without drifting.

It should answer three questions:
- what should be remembered
- what should be forgotten
- what should be reused

## Memory types

`factual`
- stable, verified facts
- example: API behavior, project invariants, limits

`episodic`
- what happened in a specific run
- example: task context, result, failure mode

`procedural`
- a reusable workflow or method
- example: a reliable sequence of steps for a task type

`caution`
- a known trap or failure pattern
- example: a bad retrieval pattern or a brittle assumption

## Candidate record shape

Every memory candidate should capture:
- type
- summary
- source run
- tags
- confidence
- utility
- recency
- verification state
- expiry

## Retrieval policy

- load only a small set of memories
- prefer memories that match the current task type
- prefer higher-confidence items
- prefer more recent items when relevance is close
- do not load expired items unless explicitly requested

## Promotion policy

A candidate should be promoted only when:
- the trace supports it
- it is not a duplicate
- the judge approves it
- it has a clear future-use condition

Suggested promotion rules:
- episodic to procedural: repeated success or a strong judge-approved pattern
- episodic to caution: repeated failure
- factual: externally verified or trace-supported and stable

## Consolidation policy

Run consolidation on a schedule:
- dedupe repeated items
- merge overlapping summaries
- rewrite weak memories into compact lessons
- expire stale items
- keep raw traces untouched

## Anti-drift rules

- If a memory is not reusable, do not promote it.
- If a memory is not verified, do not treat it as fact.
- If a memory is stale, do not load it by default.
- If a memory conflicts with current behavior, record the conflict and resolve it explicitly.

