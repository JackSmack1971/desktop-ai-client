---
description: Run a broad audit of the desktop client scaffold and its boundary contracts.
argument-hint: "[path-or-scope]"
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
---

# /audit:repo

Use this command for a broad evidence-led audit.

## Context

- Primary input: `$ARGUMENTS`
- Primary workflow: `.claude/workflows/repo-audit.js`
- Primary agents: `privacy-auditor`, `release-gatekeeper`

## Audit Order

1. Run `stack-detection`.
2. Inspect privacy and hostile-surface rules.
3. Inspect provider routing and streaming behavior.
4. Inspect storage, migrations, retention, and recovery.
5. Inspect telemetry and release evidence.

## Required Output

- Audit verdict
- Confirmed findings
- Evidence files
- Open risks
- Recommended next bounded action

