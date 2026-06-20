---
name: route-to-agent
description: Pick the right .claude/agents sub-agent for the current changed paths and delegate to it, instead of eyeballing .claude/CODEOWNERS.md by hand.
disable-model-invocation: false
user-invocable: true
---

# Route to Agent

1. Get changed paths: `git diff --name-only HEAD` (fall back to `git status --porcelain` if that's empty). If the user already named files/paths, use those instead of running git.
2. Read `.claude/CODEOWNERS.md` for the path → agent table.
3. Match each changed path against the table top-to-bottom; last matching row wins (same semantics as real CODEOWNERS). Treat the `Path` column as a prefix/glob match.
4. If changed paths resolve to more than one agent, say so and list which files go to which agent — don't silently collapse to one.
5. Invoke the resolved agent(s) via the Agent tool with the user's actual task plus the relevant file list as context.
6. If nothing matches but `*`, use `lead-engineer` per the table — don't ask the user to confirm.
