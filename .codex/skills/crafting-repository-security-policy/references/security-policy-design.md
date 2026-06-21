# Security Policy Design Rules

## Table of Contents

- [Purpose](#purpose)
- [Quality Model](#quality-model)
- [Evidence Classes](#evidence-classes)
- [Module Selection](#module-selection)
- [Supported Versions](#supported-versions)
- [Threat Model and Boundaries](#threat-model-and-boundaries)
- [Private Reporting](#private-reporting)
- [Report Quality and Anti-Noise Rules](#report-quality-and-anti-noise-rules)
- [Response Expectations](#response-expectations)
- [Coordinated Disclosure](#coordinated-disclosure)
- [Research Guidelines and Safe Harbor](#research-guidelines-and-safe-harbor)
- [Supply-Chain Claims](#supply-chain-claims)
- [Regulatory Language](#regulatory-language)
- [Maintenance](#maintenance)
- [Anti-Patterns](#anti-patterns)

## Purpose

A repository security policy is an operational interface between maintainers, researchers, users, and downstream distributors. It must reduce ambiguity and triage cost without promising capabilities the project does not possess.

The public policy should answer practical questions. Internal incident response procedures, confidential contacts, embargo lists, and remediation details belong in private runbooks unless they directly affect reporters.

## Quality Model

A strong policy is:

- **discoverable:** stored in a platform-recognized location and linked when needed;
- **private-by-default:** directs undisclosed reports to a verified private channel;
- **risk-aligned:** defines the actual trust boundaries of the software;
- **predictable:** explains handling stages and realistic response expectations;
- **noise-resistant:** requires reproducible, human-verified evidence;
- **legally cautious:** does not create unauthorized commitments;
- **machine-compatible:** references machine-readable artifacts only when they exist;
- **maintainable:** avoids stale tables and dates that cannot be supported.

## Evidence Classes

Use these classifications for every material policy statement:

- `verified`: directly supported by executable configuration, repository metadata, an existing approved policy, release records, or explicit user confirmation.
- `inferred`: strongly suggested by multiple repository signals but not formally declared.
- `unknown`: no reliable evidence exists.
- `not-applicable`: the module does not fit the project.

Only `verified` facts may be stated as current operational capabilities. `inferred` facts may shape threat-model wording but must not become hard promises. `unknown` facts require conservative language or a TODO.

Evidence priority, highest first:

1. explicit user instruction or approved organization policy;
2. active repository settings exposed through an authorized platform tool;
3. executable configuration and release automation;
4. current policy, governance, and maintainer documentation;
5. manifests, lockfiles, and source architecture;
6. README claims, badges, comments, and examples;
7. guesses based on ecosystem norms.

## Module Selection

Include a module only when it improves reporter behavior or user safety.

Required in most policies:

- support policy;
- private reporting instructions;
- scope and boundaries;
- out-of-scope categories;
- report requirements;
- response expectations;
- coordinated disclosure request;
- research guidelines;
- attribution preference.

Conditional modules:

- formal safe harbor;
- bug bounty terms;
- PGP instructions;
- CVE assignment process;
- downstream distributor coordination;
- SBOM/VEX/SLSA/signature verification;
- regulatory reporting;
- product-specific environments or asset lists.

Do not include a conditional module merely because it is considered best practice. Include it only when the project can operate it.

## Supported Versions

Choose one support model:

### Rolling or Latest-Only

Use when the project has no verified backport policy or releases frequently. State that only the latest release or current default branch receives security fixes. Avoid a version table that will become stale.

### Current Major or Minor Lines

Use when release branches, tags, changelogs, and maintenance automation show multiple supported lines. List only versions backed by evidence.

### Enterprise or Product Lifecycle

Use only when formal lifecycle dates and support commitments are published. Link the canonical lifecycle source instead of duplicating volatile dates when possible.

Rules:

- never infer EOL dates from tag age alone;
- never state that a version is patched without evidence;
- distinguish security-only maintenance from full maintenance;
- state runtime or platform support only when documented;
- tell reporters to reproduce on a supported version when practical.

## Threat Model and Boundaries

A security report should identify a crossed trust boundary, not merely a defect.

Derive boundaries from:

- untrusted inputs and parsers;
- authentication and authorization decisions;
- tenant or user separation;
- process, sandbox, IPC, plugin, and extension boundaries;
- filesystem, network, and shell permissions;
- secret and key storage;
- update, package, and release integrity;
- model/tool/data boundaries in AI systems;
- hardware and physical access assumptions;
- documented APIs and supported deployment configurations.

Good scope statements name the attacker position, protected asset, and violated guarantee. Example structure:

```text
An untrusted repository can cause the tool to execute commands outside the documented approval boundary.
```

Avoid generic lists of every OWASP category. Tailor examples to code paths and deployment modes present in the repository.

## Private Reporting

Preferred intake order:

1. verified platform-native private vulnerability reporting;
2. verified security advisory or bug bounty portal;
3. verified monitored security email, preferably with encryption guidance;
4. another authenticated private channel approved by maintainers.

Rules:

- state clearly not to open a public issue, pull request, discussion, or commit;
- give one preferred route and at most one fallback;
- verify that the route is active and monitored;
- do not invent `security@domain` addresses;
- do not publish personal contact details without approval;
- do not claim GitHub Private Vulnerability Reporting is enabled based only on a GitHub remote;
- if no secure route exists, surface a deployment-blocking TODO.

## Report Quality and Anti-Noise Rules

Require enough information to validate impact efficiently:

- affected version, commit, platform, and configuration;
- concise vulnerability description and crossed boundary;
- prerequisites and attacker capabilities;
- minimal, human-tested reproduction steps or proof of concept;
- observed result and expected secure behavior;
- impact on confidentiality, integrity, availability, authorization, or supply-chain trust;
- relevant logs with secrets and personal data removed;
- suggested remediation, if available.

Reject or redirect:

- raw scanner output without validation;
- raw fuzzer crashes without exploitability analysis or a minimized reproducer;
- speculative or theoretical claims without a feasible path;
- ordinary bugs with no security boundary crossing;
- issues affecting unsupported versions only;
- findings entirely inside an upstream dependency, unless the project integrates it unsafely;
- reports produced by automated or AI tools that the submitter has not personally verified.

Do not ban AI assistance categorically. Require the reporter to own and verify the submission.

## Response Expectations

Describe stages rather than inventing deadlines:

1. acknowledgment;
2. scope and duplicate review;
3. technical validation and severity assessment;
4. remediation and regression testing;
5. coordinated release and advisory publication;
6. credit, when desired.

Use fixed SLAs only when approved and operationally supported. For volunteer projects, use a transparent best-effort statement and invite reasonable follow-up after a stated period only if the maintainers approve that period.

Avoid:

- guaranteed patch dates;
- guaranteed CVE assignment;
- guaranteed reporter access to private forks;
- severity-specific deadlines not backed by a response program;
- promises to notify downstream distributors when no process exists.

## Coordinated Disclosure

Ask reporters to keep details private until a fix and advisory are available or a mutually agreed disclosure date is reached.

A mature internal process may include:

- private validation and patch development;
- regression and exploit tests;
- severity scoring;
- CVE request or advisory publication;
- downstream coordination;
- SBOM or VEX updates;
- synchronized release notes.

The public policy should not expose confidential coordination lists or claim these steps occur unless the project can perform them.

## Research Guidelines and Safe Harbor

Separate operational research rules from legal authorization.

Conservative research guidelines may permit:

- testing on systems and data the researcher owns or is authorized to use;
- creating the minimum data access needed to demonstrate impact;
- stopping when real user data or secrets are encountered;
- using rate limits and avoiding service disruption;
- reporting promptly and preserving confidentiality.

Prohibit:

- denial-of-service against shared or production systems;
- social engineering, phishing, or physical attacks;
- persistence, destructive actions, or lateral movement;
- accessing, copying, modifying, or deleting third-party data beyond the minimum proof;
- privacy violations or testing third-party infrastructure without permission;
- extortion or public disclosure used as leverage.

Formal safe harbor language can create legal commitments. Preserve existing approved text. When no approval exists, use non-binding research guidelines and add a TODO for legal review rather than asserting that the project will never pursue legal action.

## Supply-Chain Claims

Potential artifacts include:

- dependency update automation;
- Software Bill of Materials in SPDX or CycloneDX format;
- VEX documents;
- signed releases or container images;
- provenance attestations;
- SLSA build levels;
- reproducible-build instructions;
- release verification commands;
- CodeQL, dependency review, secret scanning, or scorecard workflows.

State a control only when the repository or release channel proves it exists. Prefer exact artifact locations and verification commands. Do not use badges as sole evidence.

Differentiate:

- `we publish`: requires an actual published artifact;
- `we generate in CI`: requires executable workflow evidence;
- `we intend to add`: belongs in a roadmap or TODO, not current policy;
- `users should verify`: must include a usable method or canonical link.

## Regulatory Language

Do not assert compliance with the EU Cyber Resilience Act, NIST guidance, ISO standards, industry regulations, or mandatory reporting timelines solely because the project follows similar practices.

Regulatory modules require:

- confirmed product and market scope;
- named legal or compliance owner;
- approved notification process;
- jurisdiction-specific review;
- separation of public researcher instructions from internal statutory reporting.

When these facts are unavailable, omit compliance claims and recommend legal review outside the public policy.

## Maintenance

Recommended review triggers:

- new major release or support-line change;
- reporting channel or maintainer ownership change;
- material architecture or deployment change;
- new package registry or distribution channel;
- addition of signing, SBOM, VEX, or provenance artifacts;
- post-incident lessons;
- at least periodic review aligned with the project's release cadence.

Prefer a review reminder in project governance or automation rather than adding a rapidly stale `last reviewed` date to the public policy.

## Anti-Patterns

Do not produce:

- a generic template with example addresses or fake keys;
- an exhaustive standards catalogue unrelated to reporter behavior;
- a version matrix guessed from package manifests;
- a public issue route for sensitive reports;
- a threat model copied from another ecosystem;
- rigid response promises for a volunteer project;
- formal legal safe harbor without authorization;
- claims that PVR, CodeQL, SLSA, SBOM, VEX, signatures, or bounties exist when unverified;
- blanket rejection of all dependency findings;
- raw AI-generated boilerplate that ignores actual architecture;
- hidden internal procedures or confidential contacts;
- a completion claim while the reporting route is unusable.
