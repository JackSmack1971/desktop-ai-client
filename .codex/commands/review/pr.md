---
description: Review a local diff or PR for merge readiness with findings-first output.
argument-hint: "[pr-url-or-number-or-local-diff]"
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
---

# /review:pr

Use this command for read-only merge readiness review.

## Context

- Primary input: `$ARGUMENTS`
- Primary workflow: `.claude/workflows/review-readiness.js`
- Primary agent: `pr-reviewer`

## Review Focus

- correctness regressions
- privacy, provider, and storage drift
- missing verification
- release evidence gaps

## Required Output

- Verdict
- Blocking findings with file evidence
- Non-blocking suggestions
- Verification gaps
- Merge safety notes

