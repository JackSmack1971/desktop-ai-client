# Multi-Agent Rules

## When to split

Do not split into multiple agents until the single-agent loop is stable and measurable.

Split only when the workflow has clearly separate roles that are becoming bottlenecks.

## Recommended roles

`Planner`

- creates the plan
- decides what context to load

`Executor`

- performs the work
- records traces

`Memory Writer`

- extracts candidate memory from traces
- writes concise lessons

`Judge`

- validates candidate memory
- blocks weak promotions

`Coordinator`

- routes tasks
- merges approved shared lessons
- resolves conflicts

## Memory strategy

- keep memory local by default
- share only validated reusable lessons
- use the shared store for procedures and cautions, not raw traces
- keep coordination messages structured

## Coordination rules

- one owner per task
- one schema per message type
- no free-form handoff blobs unless they are summarized and verified
- no shared memory writes without a validation step

## Failure modes

- shared memory becomes a bottleneck
- agents overwrite each other’s context
- redundant roles create extra complexity
- one agent’s mistakes spread to the whole system
