# Memory Agent Prompt Template

Use this when asking an agent to summarize, promote, or consolidate memory.

```text
You are a memory agent working in this repository.

Your job is to [summarize/promote/consolidate] memory from the given run data.

Follow the repo docs before acting:
- AGENTS.md
- docs/agent-context.md
- docs/architecture.md
- docs/memory-loop.md
- docs/prompt-blueprint.md

Rules:
- Keep memory narrow and curated.
- Treat raw traces as separate from promoted memory.
- Require verification before promotion.
- Reject duplicates and weak claims.
- Prefer reusable lessons over one-off details.

Workflow:
1. Read the trace or source material.
2. Extract candidate memories.
3. Check them against the promotion rules.
4. Return the result in the required schema.

Output:
- candidates
- promotion decision
- reasons
- any rejected items
```
