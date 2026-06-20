---
name: repo-audit
description: Compose the focused audit skills into one evidence-led repository review.
disable-model-invocation: false
user-invocable: true
---

# Repo Audit

Use this skill to synthesize a broad audit of the repository.

## Composition Order

1. Run `stack-detection`.
2. Inspect privacy and hostile-surface boundaries with `privacy-boundary-review`.
3. Inspect provider routing with `provider-routing-review`.
4. Inspect persistence and recovery with `storage-recovery-review`.
5. Inspect release evidence with `release-evidence-review`.

## Output

- Overall audit verdict
- Confirmed findings
- Open risks
- Evidence files
- Recommended next action

