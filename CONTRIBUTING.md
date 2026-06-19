# Contributing to desktop-ai-client

Thanks for your interest in **desktop-ai-client**, a memory-aware Tauri desktop client for long-running AI agent tasks. This guide covers setup, the quality gates your change needs to pass, and how to get a pull request merged.

## Contents

- [Before you start](#before-you-start)
- [Find or propose work](#find-or-propose-work)
- [Repository scope](#repository-scope)
- [Local setup](#local-setup)
- [Development workflow](#development-workflow)
- [Quality standards](#quality-standards)
- [Pull requests](#pull-requests)
- [Security reports](#security-reports)
- [Governance](#governance)

## Before you start

This repository has no `CODE_OF_CONDUCT`, `SECURITY.md`, `CODEOWNERS`, or `LICENSE` file yet (the [README](README.md#governance) tracks these as `[TBD]`). There is no contributor license agreement or sign-off requirement configured. Be respectful and constructive in issues and reviews regardless.

Read the nearest `AGENTS.md` before editing code in a subdirectory — this repo treats `docs/` and `.planning/` as the contract while the app is scaffolded, and several directories carry their own intent-layer notes:

- [`AGENTS.md`](AGENTS.md) — repository-wide invariants and working rules.
- [`src-tauri/AGENTS.md`](src-tauri/AGENTS.md), [`src-tauri/src/AGENTS.md`](src-tauri/src/AGENTS.md), and nested `AGENTS.md` files under `src-tauri/src/ipc/`, `src-tauri/src/providers/`, `src-tauri/src/security/`, `src-tauri/src/telemetry/` — backend module ownership.
- [`src/lib/stores/AGENTS.md`](src/lib/stores/AGENTS.md) — frontend store ownership.

If you're working with an AI coding agent, the path-scoped rule files in [`.claude/rules/`](.claude/rules/README.md) (`frontend`, `backend`, `security`, `testing`) encode the conventions enforced in review — read the narrowest one that covers your change.

## Find or propose work

- **Bugs, security findings, architecture concerns, dead code, tech debt, test gaps, or anomalies**: open an issue using the matching template under [Issues → New issue](https://github.com/JackSmack1971/desktop-ai-client/issues/new/choose). Each template (`.github/ISSUE_TEMPLATE/*.yml`) defines a specific title format (`[BUG] ...`, `[SEC] ...`, `[ARCH] ...`, `[DEAD] ...`, `[DEBT] ...`, `[TEST] ...`, `[ANOMALY] ...`, all 80 characters or fewer, describing current state rather than the proposed fix) and required fields — evidence trace, expected-vs-observed state, blast radius, reproduction steps, and a traceability footer. Fill in every required field; the template enforces this.
- **Active credential leaks**: do not file a public issue. See [Security reports](#security-reports).
- **Larger or structural changes**: open an issue first (or use the `[ARCH]` template) so the approach can be discussed before you invest time in an implementation.
- There is no separate discussion forum or support channel configured in this repository; use issues for questions too.

## Repository scope

- `src/` — SvelteKit frontend (routes, components, stores).
- `src-tauri/` — Rust/Tauri backend: IPC commands, provider routing, storage, security, telemetry. Owned per the `AGENTS.md` files inside it; keep these concerns out of the renderer.
- `docs/` — agent-facing architecture and design context. Update the relevant doc when behavior or a module boundary changes.
- `security/` — `command-inventory.toml` and `release-capabilities.toml`, the source of truth checked by `verify-command-inventory`.
- `.planning/` — phase planning and milestone artifacts.
- Generated or vendored paths are out of scope for manual edits: `node_modules/`, `.svelte-kit/`, `build/`, `dist/`, `target/`, `src-tauri/target/`, `src-tauri/gen/`, and `Cargo.lock`/`pnpm-lock.yaml` (regenerate via the package manager, don't hand-edit).

## Local setup

### Prerequisites

- Node.js 18+
- pnpm 8+ (this repo pins pnpm as its only package manager — do not add or commit a competing lockfile)
- Rust 1.77+ (`rust-version` in [`src-tauri/Cargo.toml`](src-tauri/Cargo.toml)), edition 2021
- Tauri CLI 2.x

Platform notes:

- **Windows**: install Visual Studio Build Tools, or `rustup component add` the `windows-msvc` target.
- **macOS**: `xcode-select --install`.
- **Linux**: `sudo apt install libssl-dev libgtk-3-dev libayatana-appindicator3-dev`.
- **WSL**: see the upstream Tauri WSL documentation.

### Install and verify

```bash
git clone https://github.com/JackSmack1971/desktop-ai-client.git
cd desktop-ai-client
pnpm install --frozen-lockfile
pnpm check
cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory
```

`pnpm check` runs `svelte-kit sync` plus `svelte-check`. `verify-command-inventory` checks the Rust command surface against [`security/command-inventory.toml`](security/command-inventory.toml); add new `#[tauri::command]` entries there when you introduce one.

### Run

```bash
pnpm dev      # full Tauri app, hot reload
pnpm build    # production binary
pnpm frontend:dev   # SvelteKit dev server only
```

There is no `.env.example` in this repository today — optional runtime configuration (`RUST_LOG`, provider API keys via the system keyring) is documented in the [README configuration table](README.md#configuration). Don't commit real provider keys or tokens.

## Development workflow

- Base your branch on `main` (the repository's current default branch) and target pull requests at `main`.
- There is no enforced branch-naming convention in this repository; choose a short, descriptive branch name.
- There is no enforced commit-message format; write clear, descriptive commit messages that explain why the change was made.
- Keep changes scoped — the smallest correct change that satisfies the issue, per `AGENTS.md`'s Working Rules. Update the related `docs/` file or `AGENTS.md` node when behavior or a module boundary changes.

## Quality standards

There is currently no build/test CI workflow configured. [`.github/workflows/pr-contract.yml`](.github/workflows/pr-contract.yml) only validates that a pull request body satisfies the evidence contract described below — it does not run tests, lint, or builds. Run these checks locally before opening a pull request — they are the project's actual verification surface:

| Area | Command | Notes |
| --- | --- | --- |
| Frontend type-check | `pnpm check` | Required after any `.ts`/`.svelte` change. |
| Lint | `pnpm lint` | ESLint over JS/TS/Svelte; excludes `.claude/`, `.planning/`, `src-tauri/`, and build output. |
| Format check | `pnpm format` | Prettier check (tabs, single quotes, `prettier-plugin-svelte`). Run `pnpm format:write` to fix in place. |
| Backend tests | `cargo test --manifest-path src-tauri/Cargo.toml` | Required for any `src-tauri/src/**/*.rs` change. |
| Command inventory | `cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory` | Required when you add, remove, or rename a `#[tauri::command]`. |
| Release/security-sensitive config | `pnpm tauri build` or `cargo tauri build` | Required before merging changes to `tauri.conf.json`, capabilities, permissions, or CSP, per `.claude/rules/backend.md` and `.claude/rules/security.md`. |

Targeted guidance for specific change types (Svelte components, IPC commands, SQLite migrations, provider routing, security/privacy boundaries) lives in [`.claude/rules/frontend.md`](.claude/rules/frontend.md), [`.claude/rules/backend.md`](.claude/rules/backend.md), [`.claude/rules/security.md`](.claude/rules/security.md), and [`.claude/rules/testing.md`](.claude/rules/testing.md). At minimum:

- Add at least one Rust unit test or TypeScript integration test for every new IPC command (success and error payloads).
- Add a regression test (or an explicit note on what's missing and why) for any behavior-changing fix.
- Treat security and privacy regressions as hostile by default — test the failure mode, not just the happy path.
- Don't widen data exposure (raw paths, secrets, prompt content to the renderer) without an explicit, reviewed reason.

## Pull requests

- Target `main`.
- Reference the issue your change addresses (most work should start from one of the `[BUG]`, `[SEC]`, `[ARCH]`, `[DEAD]`, `[DEBT]`, `[TEST]`, or `[ANOMALY]` templates).
- Keep the diff scoped to the issue; call out any necessary doc or `AGENTS.md` updates in the PR description.
- Opening a pull request applies [`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md) automatically. Complete every section, including the exact commands you ran from the table above with their results — `pr-contract.yml` rejects empty or unresolved sections.
- For changes that materially affect authentication, authorization, secrets, cryptography, untrusted input, command execution, network boundaries, or sandboxing, start from the [security-sensitive template](https://github.com/JackSmack1971/desktop-ai-client/compare/main...main?quick_pull=1&template=security-sensitive.md) (`.github/PULL_REQUEST_TEMPLATE/security-sensitive.md`) instead.
- For changes that produce a formal versioned release or packaged installer/artifact, start from the [release template](https://github.com/JackSmack1971/desktop-ai-client/compare/main...main?quick_pull=1&template=release.md) (`.github/PULL_REQUEST_TEMPLATE/release.md`) instead.

Pre-flight checklist:

- [ ] `pnpm check` passes
- [ ] `pnpm lint` and `pnpm format` pass (or `pnpm format:write` was run)
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes, if `src-tauri/` changed
- [ ] `cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory` passes, if a Tauri command changed
- [ ] Relevant `docs/` or `AGENTS.md` files updated, if behavior or boundaries changed
- [ ] New or updated tests cover the change, or the PR explains why that isn't possible yet

## Security reports

Do not paste active secrets, tokens, keys, credentials, or session cookies into a public issue. For active leaked credentials, use [GitHub Security Advisories](https://github.com/JackSmack1971/desktop-ai-client/security/advisories) for this repository instead of an issue. Otherwise, file using the [`[SEC]` Security Finding template](.github/ISSUE_TEMPLATE/security.yml), which includes a credential-advisory gate you must complete.

There is no separate `SECURITY.md` policy in this repository yet.

## Governance

This repository does not yet have a published `LICENSE`, `CODE_OF_CONDUCT`, or `CODEOWNERS` file — see the [README governance table](README.md#governance) for current status. There is no defined response-time commitment or maintainer escalation path beyond the issue templates above.
