# Evidence Policy

Use this policy to prevent plausible but false contribution guidance.

## Claim classes

### Hard requirements

A hard requirement uses words such as **must**, **required**, **blocked**, or **cannot merge**. Publish it only when one of these sources enforces or explicitly states it:

- Active CI, release automation, branch protection documentation, or a checked-in validator.
- A repository policy file, issue form, pull-request template, ownership file, license process, DCO/CLA configuration, or security policy.
- A task definition or configuration whose failure blocks the documented workflow.

### Recommended practice

A recommendation may be published when the repository consistently demonstrates it but does not enforce it. Label it as recommended, preferred, or typical. Examples include branch naming visible in recent work, test placement conventions, and commit wording patterns.

### Optional pathway

An optional pathway must be presented as optional. Examples include devcontainers, alternative package managers explicitly supported by CI, or targeted test commands that supplement the full suite.

## Source precedence

1. Current CI and release execution.
2. Package scripts, Make/Just/Nox/Tox tasks, workspace manifests, and tool configuration.
3. Maintainer-authored policy, templates, and governance files.
4. README and development documentation.
5. Stable patterns in representative code, tests, and recent history.
6. Existing contribution prose.

A lower-precedence source may add detail but must not contradict a higher-precedence source.

## Minimum corroboration

- **Commands:** Require a checked-in definition or a current CI invocation. Do not derive commands solely from ecosystem convention.
- **Versions:** Require a manifest, lockfile metadata, runtime-version file, container image, or CI setup action.
- **Default branch:** Require local remote metadata, current CI triggers, or repository documentation. Do not assume `main`.
- **DCO/CLA/signing:** Require explicit configuration or policy. Never add these as generic best practice.
- **Community channels:** Require an existing issue form, discussion link, support file, security policy, or verified repository URL.
- **Response times and maintainer promises:** Require an explicit service expectation in repository policy.
- **AI-assisted changes:** Require an existing repository rule before making disclosure, provenance, or prohibition mandatory.

## Conflict handling

When sources disagree:

1. Record the conflict before drafting.
2. Identify which source is current by inspecting CI references, manifest scripts, timestamps in Git history, and active templates.
3. Use the highest-precedence current source.
4. Avoid silently preserving stale prose.
5. Report the conflict and chosen resolution in the completion handoff when maintainer intent remains uncertain.

## Safe inference

Safe inference is limited to non-policy connective prose. It may explain why a verified command is run or organize verified steps into a logical sequence. It must not create new obligations, supported platforms, compatibility guarantees, legal terms, or communication channels.

## Repository archetypes

Classify the repository only to control document depth:

- **Small project:** Favor a short direct workflow and avoid governance overhead not present in the repository.
- **Published library/package:** Emphasize compatibility, public API stability, targeted tests, changelog rules, and release-facing validation when evidenced.
- **Application/service:** Emphasize environment setup, migrations, integration tests, local services, observability, and security boundaries when evidenced.
- **Monorepo:** Explain root bootstrap, workspace selection, affected-package validation, and package-specific exceptions.
- **Documentation/content:** Emphasize allowed content areas, preview/build commands, editorial style, localization, and generated content boundaries.
- **Scientific/research:** Emphasize reproducibility, environments, data provenance, citation, and contributor credit when supported.
- **Enterprise/internal:** Emphasize access boundaries, ownership, change management, required approvals, and private reporting channels without exposing secrets.
