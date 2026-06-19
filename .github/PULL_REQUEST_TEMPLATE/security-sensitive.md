<!-- managed-by: optimize-github-operations -->
<!--
Use this template for changes affecting authentication, authorization,
permissions, secrets, cryptography, untrusted input, command execution,
network boundaries, privacy, sandboxing, or sensitive data.

Do not disclose an unpatched vulnerability in a public pull request. Follow
the repository's private disclosure process instead.
-->

## Security objective

<!-- State the security property introduced, restored, or preserved. -->

## Related advisory, issue, or threat model

<!-- Use a safe reference. Do not publish restricted vulnerability details. -->

## Change summary

<!-- Describe the security-relevant behavior changes. -->

## Scope

### Included

### Not included

## Classification

| Field | Value |
| --- | --- |
| Risk | <!-- Medium / High — include the security reason --> |
| Breaking change | <!-- No / Yes — explain below --> |
| User-facing change | <!-- No / Yes — explain below --> |
| Required reviewer | <!-- Concrete user or team, or a reasoned N/A --> |

## Assets and trust boundaries

<!-- Identify protected assets, trusted components, untrusted inputs, privilege boundaries, network boundaries, and persistence boundaries. -->

## Attacker model and abuse cases

<!-- Describe realistic attacker capabilities and abuse paths, including negative and unexpected flows. -->

## Authorization and validation flow

<!-- Explain identity, authorization, validation, normalization, policy decisions, and fail-open or fail-closed behavior. -->

## Secrets and sensitive data

<!-- Address collection, storage, transport, access, redaction, logging, rotation, expiration, deletion, and accidental disclosure. -->

## Failure behavior

<!-- Describe timeout, retry, partial failure, cancellation, fallback, downgrade, and recovery. State whether any failure can weaken policy. -->

## Compatibility and migration

<!-- Describe old and new compatibility, persisted-data changes, deployment ordering, and rollback limitations. -->

## Security verification

| Verification | Evidence |
| --- | --- |
| Positive-path tests | <!-- Command or scenario and observed result --> |
| Negative authorization tests | <!-- Command or scenario and observed result --> |
| Malformed or adversarial input tests | <!-- Command or scenario and observed result --> |
| Secret or sensitive-data leakage checks | <!-- Command or scenario and observed result --> |
| Dependency or supply-chain review | <!-- Evidence or a reasoned N/A --> |
| Static or dynamic security analysis | <!-- Evidence or a reasoned N/A --> |
| Manual abuse-case verification | <!-- Scenario and observed result --> |

## Residual risk

<!-- State what remains possible and which assumptions must continue to hold. -->

## Rollback and incident recovery

<!-- Explain safe rollback, credential rotation, data remediation, feature disablement, and monitoring after deployment. -->

## Reviewer guidance

<!-- Identify security-sensitive files, suggested review order, assumptions to challenge, and required security or domain reviewer. -->

## Documentation, release, and follow-up

<!-- Identify security documentation, operational runbooks, release notes, and tracked follow-up work. -->

## Checklist

- [ ] Authorization is enforced at the trusted boundary.
- [ ] Invalid, missing, stale, replayed, and malformed inputs were considered.
- [ ] Failure and fallback behavior does not silently weaken policy.
- [ ] Logs and errors do not expose secrets or sensitive data.
- [ ] New permissions or capabilities are minimal and explicitly justified.
- [ ] Security tests include negative and adversarial cases.
- [ ] I reviewed generated or agent-assisted security-sensitive code.
- [ ] Residual risks and monitoring requirements are documented.
