---
description: Run a focused audit of secrets, file intake, telemetry, and hostile renderer surfaces.
argument-hint: "[path-or-scope]"
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
---

# /audit:privacy

Use this command for privacy and hostile-surface review.

## Context

- Primary input: `$ARGUMENTS`
- Primary workflow: `.claude/workflows/repo-audit.js`
- Primary agent: `privacy-auditor`

## Audit Order

1. Inspect secrets handling and credential boundaries.
2. Inspect file intake and raw path authority.
3. Inspect telemetry, redaction, and release evidence.
4. Inspect preview surfaces and renderer escape hatches.

## Required Output

- Privacy verdict
- Blocking findings
- Evidence files
- Redaction or sandbox gaps
- Follow-up verification

