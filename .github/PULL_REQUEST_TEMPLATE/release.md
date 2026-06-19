<!-- managed-by: optimize-github-operations -->
<!-- Use this template for formal version, packaging, signing, publication, or release-configuration changes. -->

## Release objective

<!-- State why this release is being produced and the intended audience or channel. -->

## Release identity

| Field | Value |
| --- | --- |
| Version | <!-- Exact version or immutable release identifier --> |
| Source commit | <!-- Full commit SHA from the release source --> |
| Release channel | <!-- Stable / Preview / Nightly / Internal, using the repository's actual vocabulary --> |
| Release owner | <!-- Concrete accountable user or team --> |

## Included changes

<!-- Summarize included behavior changes and link the authoritative change inventory. -->

## Excluded or deferred changes

<!-- State known exclusions and deferred release work. -->

## Artifact matrix

| Artifact or platform | Build source | Verification evidence |
| --- | --- | --- |
| <!-- Actual artifact or platform --> | <!-- Reproducible source or job --> | <!-- Observed result, checksum, or attestation --> |

## Provenance and signing

<!-- Provide build provenance, signing, checksum, SBOM, and publication evidence. Use a reasoned N/A only when the repository publishes no signed or attestable artifact. -->

## Compatibility and migration

<!-- Address supported versions, breaking changes, configuration changes, data migrations, upgrade ordering, and downgrade constraints. -->

## Risk and rollback

<!-- Describe release failure modes, kill switches, rollback, yank, deprecation, and recovery procedures. -->

## Release verification

| Verification | Evidence |
| --- | --- |
| Clean source and version check | <!-- Command and observed result --> |
| Test and quality gates | <!-- Commands or CI run and observed result --> |
| Reproducible build or package check | <!-- Command and observed result --> |
| Artifact integrity | <!-- Checksums, signatures, attestations, or reasoned N/A --> |
| Installation or upgrade smoke test | <!-- Scenario and observed result --> |
| Documentation and release notes | <!-- Concrete files or publication evidence --> |

## Post-release validation

<!-- State monitoring, smoke tests, ownership, success signals, and incident thresholds after publication. -->

## Reviewer guidance

<!-- Identify the release-critical files, suggested review order, highest-risk artifact, and required approvers. -->

## Follow-up work

<!-- Record tracked cleanup, deprecation, monitoring, or future release work. -->

## Checklist

- [ ] The version and source commit are exact and immutable.
- [ ] Every intended artifact is represented in the matrix.
- [ ] Signing, provenance, checksums, and publication permissions were reviewed.
- [ ] Upgrade, downgrade, rollback, and recovery behavior are documented.
- [ ] Release notes and user-facing documentation match the artifacts.
- [ ] Post-release validation has an accountable owner.
