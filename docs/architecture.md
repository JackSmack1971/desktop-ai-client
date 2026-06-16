# Architecture

## System shape

The system is a long-running agent with a persistent memory layer.

At minimum, it should include:
- `Planner`
- `Executor`
- `Memory Writer`
- `Memory Manager`
- `Retriever`
- `Judge`

Multi-agent coordination is optional and should come later.

## Runtime flow

### Start of run
1. Load the task.
2. Retrieve a small set of relevant memories.
3. Load active working state.
4. Build a plan.

### During the run
1. Execute steps.
2. Record tool calls and outputs.
3. Mark failures immediately.
4. Keep trace data separate from memory data.

### End of run
1. Summarize the run.
2. Extract candidate memories.
3. Judge candidate quality.
4. Promote, merge, or reject.
5. Persist the final trace and summary.

## Component boundaries

`Planner`
- breaks work into steps
- decides what context to load
- chooses when to call tools

`Executor`
- performs tool calls
- writes trace events
- reports outcomes

`Memory Writer`
- converts traces into candidate memory records
- extracts lessons, warnings, and procedures

`Memory Manager`
- dedupes memory
- scores memory
- expires stale items
- promotes verified items

`Retriever`
- loads only the most relevant memories
- limits context size
- applies recency and confidence rules

`Judge`
- validates candidate memories
- blocks weak or duplicated claims
- checks support in the trace or from external evidence

## Prompting boundary

Prompt design rules live in `docs/prompt-blueprint.md`.

Use that file whenever you are changing:
- system prompts
- developer prompts
- task prompts
- memory prompts
- routing or coordination prompts
