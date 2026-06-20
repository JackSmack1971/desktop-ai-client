---
description: Check release readiness, command inventory, and evidence completeness.
argument-hint: "[branch-or-build-target]"
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
---

# /release:readiness

Use this command before a build, publish, or tagged release.

## Context

- Primary input: `$ARGUMENTS`
- Primary workflow: `.claude/workflows/release-readiness.js`
- Primary agent: `release-gatekeeper`

## Required Output

- Readiness verdict
- Evidence present
- Evidence missing
- Blocking release concerns
- Safe next step

