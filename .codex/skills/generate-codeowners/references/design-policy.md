# CODEOWNERS design policy

## 1. Optimize the socio-technical boundary

CODEOWNERS is review routing, not a directory catalog. Optimize for weak team ownership: contributors may change any area, while a stable team acts as steward. Avoid two extremes:

- broad collective ownership that creates notification fatigue and bystander apathy;
- single-person ownership that creates absence risk, succession risk, and delivery bottlenecks.

For organization repositories, team ownership is the default. Individual ownership is an exception that requires explicit justification.

## 2. Match strategy to repository archetype

### Focused library or SDK

- Favor core maintainer or API-governance teams.
- Protect public API, release, packaging, compatibility, and generated client boundaries.
- A global fallback is optional and often unnecessary when all stable surfaces are explicit.

### Modular application

- Use domain directory ownership for product areas.
- Keep local tests with the product team.
- Assign system integration suites only when a distinct QA function exists.
- Use unowned paths sparingly for reproducible generated assets.

### Enterprise monorepo

- Use specific app, service, and package rules.
- Put platform defaults before product overrides.
- Keep shared libraries and infrastructure explicit.
- Audit broad extension rules because they often override domain ownership.

### Open-source project

- Use a verified core-maintainer fallback when contributor trust is low.
- Delegate mature components to verified component teams.
- Protect release automation, package publishing, security policy, and executable CI paths.

### Internal platform or infrastructure repository

- Assign infrastructure, policy, release, and CI/CD to platform or SRE teams.
- Use multiple listed owners to widen qualified review coverage, not to simulate multi-team approval; GitHub requires only one listed owner's approval.
- Recommend rulesets for approval counts or stricter gates outside CODEOWNERS.

## 3. Apply GitHub parser rules precisely

- GitHub searches `.github/CODEOWNERS`, then `CODEOWNERS`, then `docs/CODEOWNERS`, and uses the first file found.
- The file applies per branch and must exist on the pull request base branch.
- The file must be smaller than 3 MiB.
- Matching is case sensitive.
- The last matching rule wins.
- A rule is a pattern followed by all owners on the same line.
- Invalid lines are skipped, leaving paths unowned.
- Unsupported syntax includes negation with `!`, character ranges with `[ ]`, and escaping a pattern that starts with `#`.
- A blank-owner rule is valid and removes ownership inherited from earlier matches. Treat it as a deliberate review-policy exception.
- Owners must be existing users, verified email identities, or visible teams with explicit write access. Prefer `@organization/team`.

## 4. Order from broad to specific

Use this ordering unless repository evidence requires a documented exception:

1. optional fallback;
2. repository governance and root configuration;
3. platform, security, CI/CD, release, and infrastructure;
4. shared libraries and cross-cutting systems;
5. product applications, services, packages, or components;
6. high-risk subdirectories such as migrations or cryptography;
7. narrow exceptions and blank-owner generated paths;
8. explicit ownership of `.github/CODEOWNERS` when a later rule could otherwise override it.

Never place a catch-all after specialized rules. Avoid late file-extension patterns because they override every earlier matching domain rule.

## 5. Cross-cutting path guidance

### Protect strongly

- `.github/workflows/`, reusable workflows, actions, and release automation;
- `.github/CODEOWNERS`, repository policies, and security configuration;
- package manifests, lockfiles, build systems, root toolchain configuration;
- infrastructure-as-code, deployment manifests, Kubernetes, Terraform, and Helm;
- database schema and migration paths;
- authentication, authorization, cryptography, signing, secret-handling, and policy engines;
- public API contracts, protocol schemas, and compatibility surfaces;
- package publishing and artifact-signing configuration.

### Keep localized

- feature code and co-located unit tests;
- service-specific configuration and deployment overlays;
- package-local manifests in a monorepo;
- component documentation when product teams maintain it.

### Consider intentional unownership only when all are true

- files are generated, vendored, compiled, translated automatically, or low-risk documentation;
- the directory is reproducible from reviewed source inputs;
- manual edits are prevented or detected by CI;
- removing code-owner review does not bypass a security or release gate;
- the plan records an explicit rationale.

## 6. Evidence hierarchy

Use ownership evidence in this order:

1. verified user-supplied owner map;
2. valid existing CODEOWNERS teams;
3. GitHub API teams with visible status and write-level permission;
4. verified personal repository owner.

Do not derive handles from Git author names, commit emails, directory names, package scopes, chat mentions, or organizational role titles. Git history is evidence for boundary design and bus-factor risk only.

## 7. Anti-patterns

- Hardcoding individuals in an organization repository without an explicit exception.
- A catch-all at the bottom of the file.
- Broad extension rules after domain rules.
- Inline comments after owners.
- Empty or secret teams, teams without explicit write access, or stale handles.
- Treating multiple owners on one rule as multiple mandatory approvals.
- Adding blank-owner rules to reduce friction on critical paths.
- Creating one rule per file when a stable directory boundary exists.
- Using CODEOWNERS as the only enforcement layer without recommending branch rules or repository rulesets.

## 8. Governance recommendations outside the file

Report but do not apply:

- require pull requests before merging;
- require review from code owners;
- dismiss stale approvals when new commits are pushed where appropriate;
- configure required approval counts in rulesets or branch protection;
- use narrowly scoped bot bypasses for dependency and release automation;
- validate CODEOWNERS on changes and on a schedule;
- periodically audit stale teams, empty teams, and review latency.
