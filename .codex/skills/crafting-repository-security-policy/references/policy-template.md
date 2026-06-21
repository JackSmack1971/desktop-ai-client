# Adaptive SECURITY.md Template

## Table of Contents

- [Usage Rules](#usage-rules)
- [Core Template](#core-template)
- [Support Variants](#support-variants)
- [Reporting Variants](#reporting-variants)
- [Scope Modules](#scope-modules)
- [Response Variants](#response-variants)
- [Research and Safe-Harbor Variants](#research-and-safe-harbor-variants)
- [Supply-Chain Module](#supply-chain-module)
- [Omission Rules](#omission-rules)

## Usage Rules

This is a modular drafting aid, not text to copy blindly.

- Replace every bracketed field with verified repository-specific content.
- Delete modules that do not apply.
- Never leave sample domains, keys, versions, or URLs.
- Prefer concrete trust boundaries over generic vulnerability categories.
- Keep one top-level heading.
- Put the preferred private reporting route near the beginning.
- Do not assert an SLA, bounty, CVE process, safe harbor, or supply-chain artifact unless approved and operational.

## Core Template

```markdown
# Security Policy

## Supported Versions

[SELECT ONE SUPPORT VARIANT]

## Reporting a Vulnerability

Do not open a public issue, pull request, discussion, or commit containing an undisclosed vulnerability.

[PREFERRED VERIFIED PRIVATE REPORTING ROUTE]

[FALLBACK VERIFIED PRIVATE ROUTE, OR OMIT]

Include the following in your report:

- affected version, commit, platform, and relevant configuration;
- a concise description of the vulnerability and the security boundary it crosses;
- attacker prerequisites and required access;
- minimal, human-verified reproduction steps or proof of concept;
- observed behavior, expected secure behavior, and practical impact;
- logs or screenshots with secrets and personal data removed;
- suggested remediation, if available.

## Security Scope

This policy covers vulnerabilities in [SUPPORTED PROJECT COMPONENTS AND DISTRIBUTED ARTIFACTS] when used in [SUPPORTED DEPLOYMENT OR API MODEL].

In-scope findings include:

[REPOSITORY-SPECIFIC TRUST-BOUNDARY VIOLATIONS]

## Out of Scope

The following are normally handled as ordinary bugs, upstream reports, or unsupported configurations:

[REPOSITORY-SPECIFIC EXCLUSIONS]

A finding may still be in scope when this project introduces or materially amplifies an upstream weakness through unsafe integration, configuration, or defaults.

## Report Quality

Reports must describe a feasible exploit path and include a human-tested reproducer. Raw scanner output, untriaged fuzzer crashes, speculative risk ratings, or AI-generated reports that the submitter has not personally verified may be closed without further investigation.

## What to Expect

[SELECT RESPONSE VARIANT]

We may ask for additional information, coordinate a fix privately, and request that technical details remain confidential until a release or mutually agreed disclosure date is available.

## Coordinated Disclosure

Please allow a reasonable period for validation, remediation, regression testing, release preparation, and downstream coordination. Do not publish exploit details before coordination is complete unless an alternative disclosure date has been mutually agreed.

## Security Research Guidelines

[SELECT RESEARCH OR APPROVED SAFE-HARBOR VARIANT]

## Attribution

Tell us whether you would like public credit. Subject to your preference and any legal or privacy constraints, we will attempt to acknowledge validated reports in the advisory or release notes.

[OPTIONAL SUPPLY-CHAIN MODULE]
```

## Support Variants

### Rolling Branch

Use only when the default branch is the supported unit:

```markdown
This project follows a rolling development model. Security fixes are applied to the current default branch. Older commits, snapshots, and forks are not supported unless maintainers state otherwise.

Before reporting, confirm that the issue reproduces on the latest default-branch revision.
```

### Latest Stable Release

Use when there are releases but no verified backport policy:

```markdown
Only the latest stable release receives security fixes. Older releases may contain known issues and should be upgraded before a report is submitted.
```

### Maintained Release Lines

Use only with verified branches or lifecycle documentation:

```markdown
| Release line | Security support |
| --- | --- |
| `[VERIFIED LINE]` | Supported |
| `[VERIFIED LINE]` | Security fixes only |
| Older releases | Not supported |
```

Do not add EOL dates unless maintained in an authoritative lifecycle source.

### External Lifecycle

```markdown
Supported versions are defined in [CANONICAL LIFECYCLE DOCUMENT]. Reports affecting unsupported versions should first be reproduced on a supported release.
```

## Reporting Variants

### Verified GitHub Private Vulnerability Reporting

Use only when enablement is confirmed:

```markdown
Use GitHub Private Vulnerability Reporting from this repository's **Security** tab and select **Report a vulnerability**. This is the preferred reporting channel.
```

### Verified Security Email

```markdown
Send the report to `[VERIFIED SECURITY EMAIL]` with the subject `[PROJECT] vulnerability report`.

[OPTIONAL VERIFIED PGP KEY AND FINGERPRINT]
```

### Verified Bug Bounty or Security Portal

```markdown
Submit the report through [VERIFIED PROGRAM OR PORTAL NAME] at [VERIFIED URL]. The program rules and asset scope govern submissions made through that channel.
```

### Missing Private Route

Use only as an explicit blocker in a draft, not as a deployment-ready policy:

```markdown
[TODO: Configure and verify a private vulnerability reporting channel before publishing this policy.]
```

Do not replace the missing route with a public issue tracker.

## Scope Modules

Select and rewrite the modules that match the repository.

### AI and Agent Systems

Potential in-scope statements:

```markdown
- untrusted prompts, retrieved content, or tool responses causing actions outside the documented approval and permission boundary;
- bypass of filesystem, shell, network, browser, or tool-use restrictions enforced by this project;
- cross-user access to conversations, memory, credentials, model inputs, or generated artifacts;
- unauthorized extraction of secrets or protected model/data assets from project-managed storage;
- security-relevant cost or resource abuse that bypasses configured project controls.
```

Potential exclusions:

```markdown
- model hallucinations, subjective output quality, or generic alignment complaints without a crossed security boundary;
- behavior originating solely in an external model provider when this project does not create or amplify the issue;
- prompt-format or content-filter bypasses with no unauthorized data access, tool action, or privilege change.
```

### Web or API Service

Potential in-scope statements:

```markdown
- authentication, authorization, or tenant-isolation bypass;
- injection, request forgery, unsafe redirects, or file-processing flaws reachable through supported interfaces;
- exposure of project-managed secrets, session material, or private user data;
- bypass of rate, permission, or workflow controls that creates material security impact.
```

Potential exclusions:

```markdown
- missing security headers with no demonstrated impact;
- self-XSS or attacks requiring the victim to paste attacker-controlled code into a privileged console;
- denial-of-service tests against production or shared infrastructure;
- vulnerabilities in unsupported deployments or third-party services not controlled by the project.
```

### Desktop or Local-First Application

Potential in-scope statements:

```markdown
- renderer, webview, extension, or content process escalation into privileged backend commands;
- updater, package, signature, or release-integrity bypass;
- unauthorized local file, keychain, credential, or workspace access across documented boundaries;
- unsafe protocol handlers, deep links, IPC messages, plugins, or imported project content;
- command execution without the required user approval or policy check.
```

Potential exclusions:

```markdown
- attacks requiring prior administrator/root control of the same host unless a stronger boundary is explicitly promised;
- modifications to local binaries, configuration, or memory by an already privileged local attacker;
- unsupported shared-machine configurations when the application documents a single-user trust model.
```

### CLI or Developer Tool

Potential in-scope statements:

```markdown
- malicious repository, archive, or configuration content causing command execution outside documented consent;
- path traversal, symlink attacks, or workspace escape during file operations;
- credentials or sensitive source content exposed through logs, temporary files, or subprocess arguments;
- plugin, hook, or dependency execution that bypasses documented trust controls.
```

Potential exclusions:

```markdown
- shell aliases, environment manipulation, or configuration controlled by an already privileged local user;
- ordinary command failures or crashes without a security boundary crossing;
- unsafe invocation that contradicts documented usage and requires the user to provide attacker-chosen shell syntax directly.
```

### Library or SDK

Potential in-scope statements:

```markdown
- memory-safety, parsing, injection, or cryptographic flaws reachable through documented APIs using untrusted input;
- unsafe defaults that violate the library's documented security guarantees;
- unexpected network, filesystem, or process side effects under supported usage;
- package or release tampering affecting officially distributed artifacts.
```

Potential exclusions:

```markdown
- application-level misuse that contradicts documented API requirements;
- unsupported compiler, runtime, platform, or feature combinations;
- vulnerabilities entirely in upstream dependencies unless this library integrates or configures them unsafely.
```

### Cryptographic Software

Potential in-scope statements:

```markdown
- private-key or secret extraction across the documented attacker boundary;
- signature, authentication, randomness, or protocol failures;
- downgrade or algorithm-confusion attacks in supported configurations;
- side channels within the explicitly supported hardware and co-residency model.
```

The policy must state assumptions for physical access, same-host attackers, specialized hardware, and microarchitectural side channels.

### Embedded or IoT

Potential in-scope statements:

```markdown
- unauthorized firmware installation, secure-boot bypass, or rollback/downgrade;
- extraction of protected device keys within the documented physical-access model;
- bypass of debug-interface controls or device authentication;
- remote compromise through supported network interfaces or update paths.
```

State supported hardware revisions and whether invasive physical attacks are excluded.

## Response Variants

### Volunteer or Small Maintainer Team

```markdown
This project is maintained on a best-effort basis. We will attempt to acknowledge and assess complete reports as maintainer availability permits. Remediation timing depends on severity, complexity, release constraints, and maintainer capacity; no fixed response or patch deadline is promised.
```

### Staffed Response Team with Approved Targets

Use only with verified commitments:

```markdown
We target acknowledgment within [APPROVED TARGET], technical assessment within [APPROVED TARGET], and regular status updates until resolution. These are service targets rather than guarantees and may change during complex or multi-party coordination.
```

### Existing Formal SLA

Link the approved source instead of duplicating it when possible:

```markdown
Response and remediation targets are defined in [APPROVED SECURITY RESPONSE POLICY].
```

## Research and Safe-Harbor Variants

### Conservative Research Guidelines

Use when legal authorization is not verified:

```markdown
Conduct testing only on systems, accounts, and data you own or are explicitly authorized to use. Avoid service disruption, privacy violations, persistence, destructive actions, social engineering, and access to third-party data. Use the minimum access needed to demonstrate impact, stop if you encounter real user data or secrets, and report the issue promptly through the private channel above.

These guidelines describe expected research behavior and do not create rights or permissions beyond those provided by applicable law or the system owner's authorization.
```

### Approved Formal Safe Harbor

Use only when supplied or approved by authorized counsel/maintainers:

```markdown
[INSERT APPROVED SAFE-HARBOR TEXT VERBATIM OR WITH AUTHORIZED SURGICAL EDITS]
```

Do not synthesize a promise not to pursue legal action without approval.

## Supply-Chain Module

Include only verified, usable statements:

````markdown
## Release and Supply-Chain Verification

Official releases are distributed through [VERIFIED CHANNELS]. [VERIFIED ARTIFACTS] are published at [VERIFIED LOCATION]. Verify releases with:

```text
[VERIFIED COMMANDS]
```

Report suspected compromise of project-controlled build, signing, publication, or distribution infrastructure through the private vulnerability channel.
````

Possible verified artifacts:

- SPDX or CycloneDX SBOM;
- VEX document;
- signed tag, release, package, binary, or container image;
- SLSA provenance;
- checksum manifest;
- reproducible-build instructions.

Do not claim a SLSA level unless the project has valid provenance and evidence for that level.

## Omission Rules

Omit:

- a bug bounty section when no program exists;
- PGP instructions when no verified key exists;
- CVE assignment language when no process or CNA relationship is verified;
- downstream embargo procedures when no trusted coordination program exists;
- legal safe harbor without approval;
- supply-chain verification without published artifacts;
- compliance claims without legal review;
- version tables without a maintenance policy;
- contact names when a role-based private channel is sufficient.

A shorter truthful policy is better than a comprehensive fictional one.
