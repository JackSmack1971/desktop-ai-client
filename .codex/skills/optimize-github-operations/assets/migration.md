<!-- managed-by: optimize-github-operations -->
<!-- Use this template for persistent schema, state, protocol, configuration-format, cache, or index migrations. -->

## Migration objective

<!-- State why the migration is necessary and which persistent contract changes. -->

## Representation change

### Current representation

<!-- Describe the current schema, format, protocol, index, cache, or stored state. -->

### Target representation

<!-- Describe the target representation and invariants. -->

## Scope

### Included

### Not included

## Forward migration

<!-- Describe exact sequencing, prerequisites, ownership, deployment order, and completion criteria. -->

## Mixed-version compatibility

<!-- Explain old and new reader/writer compatibility during rolling deployment and partial adoption. -->

## Idempotency and interruption

<!-- Explain retries, duplicate execution, cancellation, partial failure, resume behavior, and concurrency controls. -->

## Data validation and reconciliation

<!-- Define preconditions, row or object counts, checksums, invariant checks, reconciliation, and post-migration acceptance. -->

## Backup and restore

<!-- State backup scope, restore test evidence, retention, and recovery-point limitations. -->

## Rollback or downgrade

<!-- Explain whether rollback is safe, lossy, time-bounded, or prohibited after a commitment point. -->

## Risk and observability

<!-- Describe data-loss, corruption, availability, latency, resource, privacy, and operational risks plus monitoring and alert thresholds. -->

## Verification

| Verification | Evidence |
| --- | --- |
| Dry run | <!-- Command or procedure and observed result --> |
| Representative-data test | <!-- Dataset characteristics and observed result --> |
| Failure and interruption test | <!-- Scenario and observed recovery --> |
| Idempotency or retry test | <!-- Scenario and observed result --> |
| Mixed-version test | <!-- Scenario and observed result, or a reasoned N/A --> |
| Backup restore test | <!-- Scenario and observed result --> |
| Post-migration invariant check | <!-- Command or query and observed result --> |

## Reviewer guidance

<!-- Identify migration entry points, highest-risk phase, irreversible boundary, and required data or operations reviewer. -->

## Documentation, release, and follow-up

<!-- Identify runbooks, operator communication, release ordering, cleanup, and tracked follow-up work. -->

## Checklist

- [ ] Forward migration ordering and ownership are explicit.
- [ ] Mixed-version behavior is tested or ruled out with a concrete reason.
- [ ] Retry, interruption, resume, and idempotency behavior are defined.
- [ ] Backup and restore evidence exists before the commitment point.
- [ ] Rollback limitations and irreversible steps are explicit.
- [ ] Post-migration invariants and monitoring thresholds are measurable.
