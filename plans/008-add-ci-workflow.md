# Plan 008: Add a CI workflow running typecheck, backend tests, and dependency audits on every PR

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- .github/workflows/ package.json src-tauri/Cargo.toml`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MED
- **Depends on**: `plans/005-add-test-coverage.md` (soft dependency — this plan works without it, but `pnpm test` won't exist as a CI step until plan 005 lands; see Step 2's conditional). `plans/007-remove-duplicate-npm-lockfile.md` (soft — if it lands first, this workflow only needs to cache/install via pnpm, simplifying Step 1 slightly; not a hard blocker either way).
- **Category**: dx
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

`.github/workflows/` currently contains only a `.gitkeep` placeholder — there is no CI at all. Every regression (a compile failure, a broken `cargo test`, a `pnpm check` type error) is only caught if a human or agent happens to run the relevant command locally before merging. There's also no automated dependency-vulnerability scanning (no `cargo audit`, no `pnpm audit` anywhere). This compounds with the project's own `.planning/v1.0-MILESTONE-AUDIT.md` finding that no phase has a passing verification artifact — adding CI turns "did anyone remember to run the checks" into a single enforced gate.

## Current state

- `.github/workflows/.gitkeep` is the only file under `.github/workflows/` — confirmed via directory listing.
- `.github/ISSUE_TEMPLATE/` already has 7 populated templates (bug, anomaly, architecture, dead-code, security, tech-debt, test-gap) — this repo already has *some* GitHub automation conventions in place, just not CI.
- `package.json` scripts (current, before this plan): `dev`, `build`, `preview`, `check`, `check:watch`, `frontend:dev`, `frontend:build`, `tauri`. No `test` or `lint` script exists yet (see `plans/005-add-test-coverage.md` for `test`, `plans/009-add-lint-and-format.md` for `lint`).
- `src-tauri/Cargo.toml`: `rust-version = "1.77"`, edition 2021. Notably, `keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }` — these are per-OS native backend features, meaning a Linux CI runner needs the Secret Service D-Bus dev libraries available (`libdbus-1-dev` on Debian/Ubuntu) for `cargo check`/`cargo test` to succeed; `rusqlite = { version = "0.31", features = ["bundled"] }` avoids needing a system SQLite library, which simplifies the Linux runner story.
- No existing workflow file to model conventions after (this is the first one) — match GitHub Actions idiomatic style (checkout → setup toolchains → cache → install → run) rather than inventing a different structure.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Frontend install | `pnpm install --frozen-lockfile` | exit 0 |
| Frontend typecheck | `pnpm check` | exit 0 |
| Backend compile check | `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | exit 0 |
| Backend tests | `cargo test --manifest-path src-tauri/Cargo.toml` | all pass |
| Rust dependency audit | `cargo audit --manifest-path src-tauri/Cargo.toml` (requires `cargo install cargo-audit` first, or use the `rustsec/audit-check` action) | no unresolved advisories at or above the configured severity |
| JS dependency audit | `pnpm audit --audit-level=high` | exit 0 (or document accepted exceptions) |

## Scope

**In scope**:
- New file: `.github/workflows/ci.yml`

**Out of scope**:
- Removing `.github/workflows/.gitkeep` — leave it; it's harmless alongside a real workflow file, and deleting placeholder files outside this plan's stated purpose is unnecessary churn.
- Setting up release/publish workflows, code signing, or `cargo tauri build` packaging in CI — this plan is scoped to PR/push verification (check + test + audit), not release automation.
- Adding a `lint`/`test` script to `package.json` if it doesn't exist yet — see the conditional logic in Step 2; this plan reads what scripts currently exist rather than assuming `plans/005`/`009` already landed.
- Any change to `src-tauri/Cargo.toml`, `package.json`, or application source — this plan only adds a workflow file.

## Git workflow

- Branch: `advisor/008-add-ci-workflow`
- Commit message: `ci: add typecheck, test, and dependency-audit workflow`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Check which scripts currently exist before writing the workflow

Read the current `package.json` `scripts` block. If `plans/005-add-test-coverage.md` has already landed, a `"test": "vitest run"` script will exist — include a `pnpm test` step in the workflow. If it has not landed yet, omit the frontend test step (do not invent a script that doesn't exist) and note in your final report that it should be added once plan 005 lands. Same logic for `"lint"` (from `plans/009-add-lint-and-format.md`) — include `pnpm lint` only if the script already exists.

### Step 2: Write `.github/workflows/ci.yml`

Create the file with this structure (adjust the conditional `pnpm test`/`pnpm lint` steps per Step 1's findings — the snippet below assumes neither has landed yet; add them back in if they have):

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

jobs:
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
      - run: pnpm install --frozen-lockfile
      - run: pnpm check
      - run: pnpm audit --audit-level=high

  backend:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - if: matrix.os == 'ubuntu-latest'
        run: sudo apt-get update && sudo apt-get install -y libdbus-1-dev
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: src-tauri
      - run: cargo check --manifest-path src-tauri/Cargo.toml --all-targets
      - run: cargo test --manifest-path src-tauri/Cargo.toml

  rust-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
          file: src-tauri/Cargo.lock
```

Notes on this structure:
- `libdbus-1-dev` is only needed on Linux for `keyring`'s `sync-secret-service` feature to compile — confirmed necessary from `Cargo.toml`'s feature list; Windows/macOS use their native credential stores (`windows-native`/`apple-native`) and don't need it.
- The `backend` job runs a 3-OS matrix because `keyring`'s per-OS native features mean a Linux-only CI run would never catch a Windows- or macOS-specific compile break — this is a deliberate choice given the dependency, not unnecessary cost.
- `rustsec/audit-check` is a maintained third-party action; if your environment cannot resolve third-party actions (e.g. an offline/air-gapped CI runner), substitute a manual `cargo install cargo-audit && cargo audit --manifest-path src-tauri/Cargo.toml` step instead — note the substitution in your final report.

**Verify**: There is no way to fully "run" a GitHub Actions workflow locally without `act` or pushing a real PR — this plan does not assume `act` is installed. Instead, validate the YAML is well-formed: `pnpm dlx js-yaml .github/workflows/ci.yml` (or any available YAML linter/parser) → parses without error. Additionally, manually run each command listed in the "Commands you will need" table locally and confirm each one's expected exit code, since those are exactly the commands the workflow will run.

## Test plan

No new application test is needed — the "test" here is the workflow itself, which can only be fully verified by an actual push/PR run. Document in your final report which commands you verified locally (per Step 2's Verify) versus which you could only verify by YAML syntax (the `uses:` action steps, which require an actual GitHub Actions runner to execute).

## Done criteria

- [ ] `.github/workflows/ci.yml` exists and is valid YAML
- [ ] Every `run:` step's command was independently verified to exit 0 locally (or documented as a known-acceptable audit finding, if `pnpm audit`/`cargo audit` surfaces a pre-existing advisory — do not silently suppress a real finding to make this plan's done criteria pass; report it instead)
- [ ] No files outside `.github/workflows/ci.yml` are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

- `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` or `cargo test` fails locally for reasons unrelated to this plan (e.g. a pre-existing compile error) — report it; do not "fix" unrelated code under a CI-workflow plan.
- `pnpm audit --audit-level=high` or `cargo audit` surfaces a real, currently-unpatched advisory in a direct or transitive dependency — STOP and report the advisory (package, version, CVE/advisory ID) rather than silently adding an ignore rule to make the workflow "pass." Whether to accept the risk or bump the dependency is a decision for the human reviewing this plan's PR, not something to resolve unilaterally here.

## Maintenance notes

- Once `plans/005-add-test-coverage.md` and `plans/009-add-lint-and-format.md` land, add `pnpm test` and `pnpm lint` steps to the `frontend` job — flagged here so the next contributor doesn't have to rediscover this dependency.
- The 3-OS backend matrix will roughly triple CI minutes versus a single-OS run — if that becomes a cost concern, consider narrowing to `ubuntu-latest` + `windows-latest` only (macOS runners are typically the most expensive) once the project has enough history to judge whether macOS-specific keyring breaks are a real, recurring risk or a theoretical one.
