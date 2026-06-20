---
description: Implement one issue as a bounded desktop-client change with explicit verification.
argument-hint: "[issue-url-or-number]"
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
  - Edit
  - Write
---

# /create:pr

Use this command for one issue or one narrowly bounded implementation target.

## Context

- Primary input: `$ARGUMENTS`
- Primary workflow: `.claude/workflows/issue-to-pr.js`
- Primary agent: `implementation-agent`

## Execution Steps

1. Parse the issue reference from `$ARGUMENTS`.
2. Read the repo instructions and nearest child `AGENTS.md` files.
3. Run `stack-detection` if the runtime shape is unclear.
4. Make the smallest correct change.
5. Update or add tests when behavior changes.
6. Verify the change and capture the exact commands used.

## Required Output

- Issue or scope worked
- Files changed
- Verification performed
- Privacy, provider, and storage impact
- Remaining risk
- Rollback notes if the change widened the surface

