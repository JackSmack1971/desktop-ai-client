# PR Triage Rubric

## Decision definitions

| Decision | Use when |
|---|---|
| `ready-for-review` | Scope is aligned, required information is present, no known blocking completeness gap exists, and substantive code review can begin. |
| `needs-author-changes` | The intent is valid, but concrete implementation, testing, documentation, policy, or scope corrections are required first. |
| `needs-information` | Missing reproduction details, acceptance criteria, rationale, risk disclosure, or other evidence prevents a reliable assessment. |
| `draft-not-ready` | The pull request is explicitly draft or states unfinished work that blocks meaningful review. |
| `duplicate` | Another pull request or issue substantially covers the same problem and solution scope. |
| `spam` | Concrete evidence shows irrelevant, deceptive, promotional, malicious, or mass low-value submission behavior. |
| `out-of-scope` | The change conflicts with documented project boundaries or belongs in another repository or subsystem. |
| `blocked` | Required evidence cannot be accessed or repository infrastructure is too inconclusive to complete triage. |

## Primary categories

Choose exactly one:

- `bug`: corrects unintended behavior or a regression
- `feature`: adds a new user-visible or operator-visible capability
- `security`: fixes or materially changes a security control or threat boundary
- `performance`: primarily improves latency, throughput, memory, storage, or resource use
- `refactor`: changes internal structure without intended behavior change
- `chore`: maintenance with no direct product behavior change
- `dependency`: adds, removes, or updates third-party dependencies
- `documentation`: changes documentation as the principal deliverable
- `tests`: changes test coverage or test infrastructure as the principal deliverable
- `ci`: changes continuous integration, release, build, or automation pipelines
- `build`: changes compilation, packaging, or developer build tooling

Secondary categories may capture cross-cutting effects. Never use more than two.

## Priority

| Priority | Criteria |
|---|---|
| `P0` | Active exploitation, data loss, widespread outage, release-blocking corruption, or another immediate emergency requiring interruption of normal work. |
| `P1` | Severe regression, high-impact security weakness, major production impairment, or time-critical release risk with a credible near-term deadline. |
| `P2` | Normal-priority bug, feature, maintenance, or quality improvement with meaningful value but no emergency condition. |
| `P3` | Low-impact cleanup, minor documentation, cosmetic behavior, speculative improvement, or work that can safely wait. |

Default to `P2` when impact and urgency do not justify another level. A maintainer request for urgency is evidence only when supported by project impact.

## Dimension scoring

Score each dimension `pass`, `concern`, `fail`, or `unknown`:

1. `scope_alignment`
2. `problem_definition`
3. `implementation_completeness`
4. `test_completeness`
5. `documentation_completeness`
6. `contribution_compliance`
7. `ci_health`
8. `duplicate_risk`
9. `spam_risk`
10. `reviewability`

Interpretation:

- `pass`: evidence supports readiness for this dimension
- `concern`: non-blocking weakness or review focus area
- `fail`: concrete blocking defect or unmet documented requirement
- `unknown`: evidence is unavailable or applicability cannot be established

## Confidence

- `high`: repository policy, pull request evidence, diff, and checks support the disposition with no material unresolved ambiguity
- `medium`: conclusion is supported but one or more material facts are inferred or unavailable
- `low`: major evidence gaps or conflicting project guidance remain

## Evidence quality

Prefer evidence in this order:

1. repository policy and architecture documents
2. pull request diff and changed-file inventory
3. automated checks and tests
4. linked issue acceptance criteria
5. maintainer comments and review history
6. author assertions
7. inference

A higher-ranked source overrides a conflicting lower-ranked source unless the higher-ranked source is obsolete or explicitly superseded.
