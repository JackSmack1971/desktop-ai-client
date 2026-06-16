# Prompt Blueprint

This document defines how we design prompts for coding agents in this repository.

## Goal

Prompts should help agents:
- stay on task
- use tools correctly
- preserve context across long runs
- verify before they promote memory or claim success
- avoid unstructured drift

## Core principles

### 1. Give the agent one job at a time

Each prompt should have a clear mission, a clear scope, and a clear output shape.

Bad:
- "Handle everything related to the project."

Good:
- "Summarize the run trace into candidate memory records."
- "Plan the next task and name the retrieval inputs."

### 2. Separate instructions from policy

Keep the main prompt short and task-specific.

Put stable rules in:
- AGENTS files
- architecture docs
- memory docs
- prompt blueprint

Do not duplicate the same rule in many prompts unless the rule is critical and local to that task.

### 3. Use explicit steps

When an agent must think through a process, make the sequence visible.

Preferred shape:
1. read context
2. pick the task
3. act
4. verify
5. record the result

### 4. Add guardrails at decision points

Any prompt that can cause drift should define:
- what not to do
- when to stop
- when to ask for help
- what evidence is required before promotion

### 5. Prefer structured outputs

Use simple schemas for anything another agent or tool must consume.

Examples:
- JSON
- bullet lists with fixed headings
- short tables
- typed fields

Avoid free-form prose when downstream automation depends on the result.

### 6. Make retrieval narrow

If the prompt uses memory, tell the agent to load only the most relevant items.

Do not tell the agent to "review everything."

### 7. Verify before you promote

Any prompt that extracts lessons, skills, or reusable procedures should require a check:
- supported by trace
- not a duplicate
- useful later
- safe to reuse

### 8. Keep the system prompt stable

System prompts should define identity, boundaries, and non-negotiables.

User prompts should define the current task.

Do not mix those layers unless there is a strong reason.

## Prompt layers

### System prompt

Use for:
- role
- scope
- safety
- invariants
- output contract

### Developer prompt

Use for:
- repo-specific behavior
- formatting rules
- tool usage rules
- memory and verification expectations

### Task prompt

Use for:
- the current unit of work
- current inputs
- desired output
- immediate constraints

## Prompt patterns we want

### Planning prompt

Purpose: decide the next action and the evidence needed.

Must include:
- current objective
- available context
- output format
- stop conditions

### Execution prompt

Purpose: do the actual work and report results.

Must include:
- task scope
- allowed tools
- expected artifacts
- verification requirement

### Memory prompt

Purpose: turn a run into durable knowledge.

Must include:
- candidate memory types
- promotion rules
- confidence or verification threshold
- duplicate handling

### Coordination prompt

Purpose: move work across agents without losing context.

Must include:
- owner
- handoff format
- acceptance criteria
- rollback path

## Anti-patterns

- giant all-purpose prompts
- hidden objectives
- repeated rules with no owner
- vague "use your judgment" instructions where a format is needed
- asking an agent to remember everything
- asking an agent to self-improve without a verification gate

## Prompt design checklist

Before shipping a prompt, verify:
- [ ] the job is singular
- [ ] the output shape is explicit
- [ ] the agent knows when to stop
- [ ] the retrieval scope is narrow
- [ ] verification is required where needed
- [ ] shared rules live in docs, not only in the prompt
- [ ] the prompt is shorter than it needs to be

## Default template

Use this as the starting shape for most agent prompts:

```text
You are responsible for [single job].

Use only the context relevant to this task.
If evidence is missing, say so.
Do not promote memory unless it is verified.
Follow the output format exactly.

Steps:
1. Read the relevant context.
2. Perform the task.
3. Verify the result.
4. Return the output in the required format.

Output:
[fixed schema]
```

## When to update this blueprint

Update this file when we discover:
- a recurring prompt failure
- a better output shape
- a new memory or verification pattern
- a new coordination pattern

