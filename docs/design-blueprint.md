# Design Blueprint

This document defines how we want design agents to reason about UI, layout, and visual direction in this repository.

## Goal

Design prompts should help agents:

- create intentional layouts
- avoid generic UI patterns
- preserve the repo’s visual direction
- keep agent-generated designs grounded in the actual task
- make output easy to build and easy to review

## Core principles

### 1. Pick a clear visual direction

Do not ask for "clean" or "modern" as the only direction.

Instead define:

- mood
- typography direction
- color direction
- density
- motion level
- layout shape

### 2. Make the design do one job

Each design prompt should target one outcome:

- dashboard
- landing page
- prompt library
- documentation UI
- settings flow
- onboarding flow

Do not ask for multiple unrelated surfaces in one prompt unless the user explicitly wants a system-wide redesign.

### 3. Preserve product reality

The design should reflect the actual workflow and constraints of the system.

If the agent is designing for:

- memory management
- prompt generation
- agent coordination
- task execution

then the UI should reveal those boundaries clearly.

### 4. Use hierarchy on purpose

The most important thing should be the easiest thing to see.

Design prompts should define:

- primary action
- secondary actions
- supporting context
- destructive or risky actions

### 5. Avoid generic agent UI

Do not default to:

- bland card grids
- samey sidebars
- generic purple gradients
- low-contrast dashboards
- meaningless AI sparkle language

### 6. Keep motion meaningful

Use motion only when it communicates state:

- loading
- transition
- selection
- success
- change in mode

Do not add motion just to look sophisticated.

### 7. Make systems explain themselves

Design should show:

- what the agent is doing
- what memory it used
- why a decision was made
- what needs review

### 8. Favor clarity over decoration

An interface is good if someone can understand it quickly and act on it.

Decorative choices are allowed only when they support the task.

## Design prompt layers

### System prompt

Use for:

- design role
- style boundaries
- quality bar
- anti-patterns

### Developer prompt

Use for:

- repo-specific UI structure
- component constraints
- tokens and theming
- accessibility requirements

### Task prompt

Use for:

- the exact screen or artifact to design
- the goal of the design
- the output format
- the target user

## Design prompt checklist

Before shipping a design prompt, verify:

- [ ] the screen has one main job
- [ ] the visual direction is explicit
- [ ] the agent knows what not to do
- [ ] key states are described
- [ ] hierarchy is clear
- [ ] accessibility is considered
- [ ] the prompt names the intended audience

## Default template

Use this as the starting shape for most design-agent prompts:

```text
You are responsible for designing [screen/artifact].

Build for the actual workflow, not a generic AI dashboard.
Choose a clear visual direction and keep it consistent.
Make the primary action obvious.
Show the system state and the important boundaries.

Design constraints:
- [constraint 1]
- [constraint 2]
- [constraint 3]

Output:
[fixed schema or file type]
```

## Anti-patterns

- generic dashboards
- placeholder visual language
- too many colors without a reason
- decorative complexity that hides function
- disconnected surfaces with no hierarchy
- prompt text that never explains the workflow

## When to update this blueprint

Update this file when we discover:

- a new UI pattern that works well for agents
- a visual failure mode
- a new accessibility requirement
- a new component or screen family
