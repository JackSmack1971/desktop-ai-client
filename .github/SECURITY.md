# Security Policy

## Reporting a Vulnerability

Do not open a public issue, pull request, discussion, or commit for an undisclosed vulnerability.

TODO: configure and verify a private vulnerability reporting channel before publishing this policy.

Include:

- affected version, commit, platform, and configuration;
- a concise description of the vulnerability and the security boundary it crosses;
- attacker prerequisites and required access;
- minimal, human-verified reproduction steps or proof of concept;
- observed behavior, expected secure behavior, and practical impact;
- sanitized logs, screenshots, or artifacts;
- suggested remediation, if available.

## Supported Versions

This project follows a rolling development model. Security fixes are applied to the current default branch (`main`). Older commits, snapshots, and forks are not supported unless maintainers state otherwise.

Please reproduce issues on the latest `main` revision when possible.

## Security Scope and Boundaries

This policy covers the desktop client, Tauri backend, IPC command surface, provider routing, local persistence, file intake, credential handling, and release-selected artifacts when used as shipped.

In scope:

- renderer-to-backend privilege escalation through IPC or capability checks;
- bypass of the reviewed command inventory or window restrictions;
- unauthorized access to local files, file tokens, workspace data, or persisted history;
- exposure or misuse of provider credentials stored in the backend or system keyring;
- provider-routing or privacy-mode failures that weaken the documented model boundary;
- unsafe command execution, unsafe deep-link or dialog handling, or command-policy bypass;
- SQLite persistence, retention, or FTS flaws that expose or corrupt user data;
- release-integrity issues in project-controlled build, packaging, or update paths.

## Out of Scope

Normally out of scope:

- ordinary bugs, crashes, or UI issues with no security boundary crossing;
- findings only in unsupported versions, forks, or local modifications;
- upstream dependency flaws that the project does not integrate or amplify unsafely;
- reports based only on scanner output, raw fuzzer crashes, or unverified AI-generated findings;
- model quality complaints, hallucinations, or provider behavior outside project control unless they cross a project boundary;
- attacks requiring prior root or administrator control of the same host unless the project promises a stronger boundary;
- denial-of-service testing against shared or production services.

A project bug can still be in scope when this repo introduces or amplifies an upstream issue through unsafe integration, defaults, or routing.

## Report Requirements

Please include:

- exact target and version or commit;
- what boundary is crossed;
- prerequisites and attack path;
- minimal reproduction steps;
- observed and expected behavior;
- practical impact;
- sanitized evidence.

## What to Expect

This project is maintained on a best-effort basis. We will attempt to acknowledge and assess complete reports as maintainer availability permits. Remediation timing depends on severity, complexity, release constraints, and maintainer capacity; no fixed response or patch deadline is promised.

We may ask for additional information, coordinate a fix privately, and request that technical details remain confidential until a release or mutually agreed disclosure date is available.

## Coordinated Disclosure

Please keep details private until a fix is available or a disclosure date is agreed.

## Security Research Guidelines

Conduct testing only on systems, accounts, and data you own or are explicitly authorized to use. Avoid service disruption, privacy violations, persistence, destructive actions, social engineering, and access to third-party data. Use the minimum access needed to demonstrate impact, stop if you encounter real user data or secrets, and report the issue through the private channel above.

These guidelines describe expected research behavior and do not create rights or permissions beyond those provided by applicable law or the system owner's authorization.

## Attribution

Tell us whether you want public credit. Subject to your preference and any legal or privacy constraints, we will try to acknowledge validated reports in release notes or the advisory.
