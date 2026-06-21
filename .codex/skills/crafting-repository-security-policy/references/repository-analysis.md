# Repository Analysis Guide

## Table of Contents

- [Purpose](#purpose)
- [Analysis Order](#analysis-order)
- [High-Signal Files](#high-signal-files)
- [Evidence Classification](#evidence-classification)
- [Ecosystem Heuristics](#ecosystem-heuristics)
- [Domain Heuristics](#domain-heuristics)
- [Support Model Analysis](#support-model-analysis)
- [Reporting Route Analysis](#reporting-route-analysis)
- [Security Control Analysis](#security-control-analysis)
- [Existing Policy Update Rules](#existing-policy-update-rules)
- [Final Evidence Matrix](#final-evidence-matrix)

## Purpose

Use repository evidence to decide what the public security policy can truthfully say. The bundled profiler narrows the search; it does not replace judgment or direct inspection of high-signal files.

## Analysis Order

1. Resolve the repository root and remote host.
2. Locate existing security, governance, release, and support documents.
3. Identify languages, runtimes, package ecosystems, deployment modes, and distribution channels.
4. Map trust boundaries and privileged operations.
5. Determine the release/support model.
6. Verify private reporting routes and response ownership.
7. Identify security and supply-chain controls.
8. Classify every proposed claim.

## High-Signal Files

Inspect these when present:

- `SECURITY.md`, `.github/SECURITY.md`, `docs/SECURITY.md`
- `README*`, `CONTRIBUTING*`, `GOVERNANCE*`, `SUPPORT*`, `CODEOWNERS`
- package manifests and lockfiles
- `.github/workflows/*`, `.gitlab-ci.yml`, other CI configuration
- `.github/dependabot.yml`, Renovate configuration, CodeQL and scorecard workflows
- release manifests, changelogs, release scripts, signing and provenance configuration
- `Dockerfile*`, Compose, Helm, Kubernetes, Terraform, packaging files
- Tauri, Electron, browser extension, mobile, firmware, or embedded configuration
- authentication, authorization, secrets, update, plugin, sandbox, IPC, shell, and filesystem modules

Do not execute project scripts merely to inspect the repository.

## Evidence Classification

### Verified

Examples:

- an existing policy lists a monitored contact and the user confirms it remains valid;
- an authorized repository tool confirms private vulnerability reporting is enabled;
- release workflows generate and upload an SBOM;
- signed release verification steps are documented and reproducible;
- maintenance branches and lifecycle docs define supported versions.

### Inferred

Examples:

- a Tauri application likely has IPC and local filesystem boundaries;
- a package manifest and published registry metadata suggest a library distribution model;
- an AI agent framework with shell tools likely has tool-permission and secret-exposure risks.

Use inferred facts to shape scope examples, not to assert operational guarantees.

### Unknown

Examples:

- whether GitHub PVR is enabled;
- whether `security@example.com` is monitored;
- whether maintainers can meet a 48-hour SLA;
- whether a bug bounty exists;
- whether legal safe-harbor language is approved.

Unknown operational facts must become TODOs, conservative wording, or omitted modules.

## Ecosystem Heuristics

### JavaScript and TypeScript

Look for:

- `package.json`, lockfiles, workspaces, npm publishing configuration;
- Node server frameworks, browser code, Electron, Tauri frontends, build plugins;
- lifecycle scripts and install-time execution;
- dependency confusion, prototype pollution, unsafe deserialization, XSS, SSRF, command injection, and package publish permissions.

### Python

Look for:

- `pyproject.toml`, `requirements*.txt`, `setup.py`, lockfiles;
- CLI entry points, web frameworks, notebooks, native extensions, package publishing;
- unsafe deserialization, shell invocation, path traversal, dependency confusion, archive extraction, and secret handling.

### Rust and Go

Look for:

- `Cargo.toml`, `go.mod`, workspace structure, build tags, unsafe blocks, cgo or FFI;
- network services, parsers, command execution, file permissions, update logic;
- memory-safety escapes, unsafe code assumptions, command injection, parser resource exhaustion, and release binary provenance.

### Java, Kotlin, C, and C++

Look for:

- build toolchains, compiler flags, native libraries, deserialization, JNI, memory allocators;
- supported platform and compiler matrices;
- buffer boundaries, use-after-free, integer overflow, unsafe deserialization, and build-chain integrity.

### Package and Registry Projects

Look for:

- npm, PyPI, crates.io, Maven, NuGet, OCI, Homebrew, or other publication automation;
- trusted publishers, release tokens, provenance, signing, and namespace ownership;
- malicious package substitution, dependency confusion, compromised maintainers, and release tampering.

## Domain Heuristics

### AI and Agent Systems

Evidence:

- model provider SDKs, prompt pipelines, tool registries, MCP, function calling, vector stores, model files, autonomous loops.

Potential boundaries:

- prompt or retrieved content causing unauthorized tool actions;
- tool permission escalation;
- filesystem, shell, network, or browser sandbox bypass;
- secret or model-data exfiltration;
- cross-user memory or conversation isolation;
- unsafe code execution and cost abuse.

Normally out of scope unless a boundary is crossed:

- hallucinations, subjective quality, generic jailbreaks, harmless prompt-format bypasses, or model-provider behavior outside project control.

### Web and API Services

Evidence:

- route definitions, auth middleware, database access, tenancy, sessions, uploads, proxies.

Potential boundaries:

- authentication or authorization bypass;
- tenant isolation failure;
- injection, SSRF, request smuggling, unsafe redirects;
- session fixation or token leakage;
- unsafe file processing and secret exposure.

### Desktop and Local-First Applications

Evidence:

- Tauri, Electron, native shells, updater configuration, IPC commands, filesystem APIs, keychain use.

Potential boundaries:

- renderer-to-backend privilege escalation;
- command inventory bypass;
- unsafe deep links or protocol handlers;
- updater signature bypass;
- local file or credential exposure;
- plugin trust and multi-user host assumptions.

### CLI and Developer Tools

Evidence:

- command parsers, Git hooks, repository scanning, build orchestration, archive extraction, plugin loading.

Potential boundaries:

- malicious repository content triggering commands;
- workspace escape or symlink traversal;
- credential leakage into logs;
- unsafe temporary files;
- archive extraction outside destination;
- plugin or config code execution.

### Libraries and SDKs

Evidence:

- exported APIs, package publication, examples, compatibility matrices.

Potential boundaries:

- untrusted input parsing;
- unsafe defaults;
- memory and type safety;
- cryptographic misuse;
- unexpected network or filesystem side effects.

Limit scope to documented API use and supported configurations unless the project states otherwise.

### Cryptography

Evidence:

- cryptographic primitives, key stores, TLS, signatures, wallets, password hashing.

Potential boundaries:

- key extraction, signature forgery, randomness failure, downgrade, protocol violation, side channels.

Explicitly state physical, co-resident, hardware, and microarchitectural assumptions when relevant.

### Embedded and IoT

Evidence:

- firmware, PlatformIO, Arduino, Zephyr, OTA updates, bootloaders, JTAG/UART references.

Potential boundaries:

- secure boot and firmware authenticity;
- debug-port access controls;
- hardware key storage;
- OTA rollback or downgrade;
- device identity and local physical access.

Specify hardware revision and physical-access assumptions.

## Support Model Analysis

Use direct evidence:

- release branches and tags;
- changelog and release cadence;
- backport labels or automation;
- lifecycle documentation;
- package registry versions;
- explicit maintenance statements.

Do not infer support from semantic version numbers alone.

Choose:

- `rolling`: current default branch only;
- `latest_release`: newest stable release only;
- `maintained_lines`: explicitly listed branches or release lines;
- `external_lifecycle`: canonical product lifecycle document;
- `unknown`: unresolved.

## Reporting Route Analysis

The local repository can reveal instructions but usually cannot prove remote settings. Treat PVR as verified only when:

- an authorized platform integration confirms it;
- the user confirms it;
- an existing approved policy and operational repository configuration are both available.

A GitHub remote alone proves only hosting, not PVR enablement.

For email routes, verify the exact address from approved policy or user input. Do not derive it from the repository owner or domain.

## Security Control Analysis

Classify controls by evidence:

- **Configured:** executable workflow or config exists.
- **Published:** release artifacts or canonical links exist.
- **Mentioned:** documentation claims it exists, but implementation is not verified.
- **Absent/unknown:** no reliable evidence.

Potential controls:

- dependency update automation;
- dependency review;
- static analysis and CodeQL;
- secret scanning or push protection;
- branch protection and required review;
- signed commits or releases;
- SBOM generation and publication;
- VEX publication;
- SLSA provenance;
- container/image signatures;
- OpenSSF Scorecard or Allstar.

The public policy should generally mention only published or directly usable controls.

## Existing Policy Update Rules

Preserve:

- verified private contacts;
- approved legal language;
- correct support commitments;
- project-specific threat boundaries;
- existing attribution preferences.

Correct:

- stale version tables;
- public reporting instructions;
- fake examples or placeholders;
- generic, irrelevant vulnerability lists;
- unsupported claims;
- conflicting response timelines;
- overbroad exclusions that dismiss integration flaws.

## Final Evidence Matrix

Before drafting, prepare a matrix like:

```text
Claim: GitHub PVR is the preferred intake route
Class: unknown
Evidence: GitHub remote only
Action: TODO; do not assert enabled

Claim: Latest release only is supported
Class: inferred
Evidence: no maintenance branches; rolling changelog
Action: use conservative latest-release wording

Claim: Release artifacts include CycloneDX SBOM
Class: verified
Evidence: .github/workflows/release.yml uploads *.cdx.json
Action: include exact artifact location and verification guidance
```

Do not draft until every material claim has an action.
