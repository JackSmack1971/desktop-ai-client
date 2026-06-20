# GitHub Folder Optimization Contract

## Decision hierarchy

1. Preserve repository-specific governance that is stricter and still valid.
2. Establish one automatically applied universal pull-request template.
3. Add specialized full-replacement templates only for materially different evidence contracts.
4. Use automation to validate repeatable facts; use templates to collect judgment and evidence.
5. Route reviewers with verified `CODEOWNERS`; never simulate ownership with placeholders.
6. Preserve unrelated workflows, issue templates, funding files, support files, and security policy files.

## Canonical layout

```text
.github/
├── PULL_REQUEST_TEMPLATE.md
├── PULL_REQUEST_TEMPLATE/
│   ├── security-sensitive.md
│   ├── release.md
│   └── migration.md
├── CODEOWNERS
└── workflows/
    └── pr-contract.yml
```

The default template is mandatory for ordinary pull requests. Specialized templates are exceptional entry points and must repeat essential universal sections because GitHub does not merge a selected specialized template with the default template.

## Universal PR evidence contract

The default template must collect information a reviewer cannot reliably derive from the diff:

- purpose and related context;
- externally meaningful behavior changes;
- included scope and explicit non-goals;
- change type, risk, breaking-change status, and user-facing impact;
- important design decisions and altered contracts;
- failure modes, compatibility, security, data, performance, deployment, and rollback concerns;
- reproducible verification commands and observed results;
- regression coverage;
- reviewer starting point, highest-risk area, and desired feedback;
- user-facing evidence where relevant;
- documentation, release, migration, and follow-up work;
- final author attestations.

### Evidence over promises

Reject bare statements such as “tests pass” or “I tested this.” Require the command or scenario and its observed result. `N/A` is acceptable only with a concrete reason.

### Facts versus attestations

Use prose and tables for facts, classifications, commands, results, affected components, and rollback plans. Use checkboxes only for attestations that cannot be generated reliably by CI, such as final-diff review and absence of unrelated changes or secrets.

### Scope boundaries

Require both included scope and deliberate non-goals. Non-goals are part of the review contract and must not be removed as “empty” sections.

### Reviewer routing

Require a suggested starting point, highest-risk area, and requested feedback. Do not ask authors to manually list every changed file or assigned reviewer when GitHub already provides that data.

## Specialized template thresholds

### Security-sensitive

Create when changes materially affect authentication, authorization, permissions, secrets, cryptography, untrusted input, command execution, network boundaries, data collection, privacy, or sandboxing.

Require:

- security objective;
- protected assets and trust boundaries;
- attacker capabilities and abuse cases;
- authorization, validation, and fail-open or fail-closed behavior;
- secret and sensitive-data lifecycle;
- timeout, retry, fallback, downgrade, cancellation, and recovery behavior;
- compatibility and migration implications;
- positive, negative, malformed-input, leakage, supply-chain, analysis, and manual abuse-case evidence;
- residual risk;
- safe rollback and incident recovery;
- security-sensitive files, review order, assumptions to challenge, and required domain reviewer.

Do not place unpatched vulnerability details in a public PR body.

### Release

Create when the repository publishes formal versions, packages, binaries, images, installers, signed artifacts, or deployment bundles.

Require:

- exact release identity and source commit;
- included change set and exclusions;
- artifact and platform matrix;
- build, signing, provenance, checksum, and publication evidence;
- compatibility and migration notes;
- rollback or yank procedure;
- release owner;
- pre-release and post-release validation.

### Migration

Create when persistent data, schemas, protocols, configuration formats, caches, indexes, or serialized state change.

Require:

- old and new representation;
- forward migration and deployment ordering;
- mixed-version compatibility;
- idempotency, retries, interruption, and resume behavior;
- data validation and reconciliation;
- backup and restore requirements;
- downgrade or rollback limitations;
- dry-run, representative-data, failure-injection, and post-migration evidence.

### Dependency updates

Do not create a dedicated dependency template for routine automated updates. Consider one only for manual, high-impact upgrades with materially different review evidence. The default contract can ordinarily capture version, changelog, transitive, license, vulnerability, API, lockfile, and rollback evidence.

## Templates to consolidate

The following filenames usually represent redundant change-class templates and should be mapped into the default contract rather than multiplied:

- `bug-fix.md`
- `bugfix.md`
- `feature.md`
- `refactor.md`
- `docs.md`
- `documentation.md`
- `test.md`
- `tests.md`
- `chore.md`

Do not delete them automatically. Report them and preserve history until a maintainer approves consolidation.

## CODEOWNERS contract

- Accept owner tokens only when they begin with `@` and identify a concrete GitHub user or team.
- Reject generic values such as `@owner`, `@maintainer`, `@team`, `@org/team`, or any value containing `TODO`.
- Preserve comments that explain ownership boundaries.
- Prefer narrow, architecture-aligned rules over one universal wildcard when reliable ownership evidence exists.
- Do not convert commit authors, package authors, or email addresses into GitHub handles.

## PR validator contract

The workflow must:

- define explicit read-only permissions;
- run on PR body lifecycle events;
- use the trusted base-branch workflow definition;
- avoid checkout, builds, package installation, and execution of pull-request code;
- avoid secrets and write permissions;
- parse the event payload with a standard-library runtime available on the hosted runner;
- recognize default, security, release, and migration template profiles;
- strip HTML comments before assessing completion;
- reject unresolved instructional tokens and empty required sections;
- require recognized change type and risk values for the default profile;
- require concrete verification evidence or a reasoned `N/A`;
- emit actionable errors and a nonzero exit status.

## Discoverability

When specialized templates are added, expose sanctioned links through an existing `CONTRIBUTING.md` only when such a document exists or the user separately requested its creation. Use repository-specific PR creation URLs and the `template` query parameter. Do not fabricate an owner or repository slug when the Git remote is unavailable.

## Completion evidence

A successful optimization records:

- pre-change inventory;
- selected target layout and selection rationale;
- changed and preserved files;
- backup location for replacements;
- validator result;
- workflow syntax result when `actionlint` is available;
- `git diff --check` result;
- unresolved ownership or policy blockers.
