---
name: release-evidence-review
description: Inspect release inventory, command exposure, and evidence completeness before a publish decision.
disable-model-invocation: false
user-invocable: true
---

# Release Evidence Review

Inspect:

- command inventory completeness
- security, provider, storage, and adversarial-fixture evidence
- packaging and release surfaces
- migration or destructive-data risk
- telemetry or preview leakage in release artifacts

Output:

- Release verdict
- Evidence present
- Evidence missing
- Blocking release concerns

