# Plan 012: Make the documented backend test command work on a clean checkout

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report -- do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 5f190a8..HEAD -- src-tauri/build.rs src-tauri/tauri.conf.json src-tauri/src/main.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `5f190a8`, 2026-06-19

## Why this matters

The repo tells contributors to run `cargo test --manifest-path src-tauri/Cargo.toml`
as part of the backend verification path, and plan `008` uses that command for
CI. On a clean checkout, that command currently panics before the tests run
because Tauri's `generate_context!()` check cannot find `../dist`. `cargo test
--manifest-path src-tauri/Cargo.toml --lib` does work, so the failure is not in
the library code; it is the bootstrap path assuming a frontend build artifact
already exists.

The smallest fix is to make the build script create the expected `dist/`
directory before Tauri's context macro validates it. `dist/` is already ignored
by git, so this should stay a build-time bootstrap step, not a committed
placeholder file or a `.gitignore` exception.

## Current state

- `src-tauri/tauri.conf.json:6-9` points Tauri at the frontend build output:
  ```json
  "build": {
    "beforeDevCommand": "npm run frontend:dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run frontend:build",
    "frontendDist": "../dist"
  }
  ```
- `src-tauri/src/main.rs:80` calls `tauri::generate_context!()` at process
  startup:
  ```rust
  .run(tauri::generate_context!())
  ```
- `.gitignore:7` ignores `dist/`, so a committed placeholder under that path is
  not the right shape for the fix.
- `src-tauri/build.rs:17-53` already resolves the workspace root, reads the
  reviewed command inventory, and then calls `tauri_build::build()`:
  ```rust
  let workspace_root = resolve_workspace_root(&manifest_dir);
  let inventory_path = workspace_root.join("security").join("command-inventory.toml");
  ...
  tauri_build::build();
  ```
  The script does not currently create the frontend dist directory.
- Observed behavior on this branch:
  `cargo test --manifest-path src-tauri/Cargo.toml` fails with the Tauri panic
  `The frontendDist configuration is set to "../dist" but this path doesn't exist`.
  `cargo test --manifest-path src-tauri/Cargo.toml --lib` passes.

## Commands you will need

| Purpose       | Command                                               | Expected on success |
| ------------- | ----------------------------------------------------- | ------------------- |
| Compile check  | `cargo check --manifest-path src-tauri/Cargo.toml`    | exit 0              |
| Library tests  | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | exit 0              |
| Full backend tests | `cargo test --manifest-path src-tauri/Cargo.toml` | exit 0              |

## Scope

**In scope** (the only files you should modify):

- `src-tauri/build.rs`

**Out of scope** (do NOT touch, even though related):

- `src-tauri/tauri.conf.json` -- keep the configured `frontendDist` path as-is.
  The issue is that the path is missing on a clean checkout, not that the path
  value is wrong.
- `src-tauri/src/main.rs` -- do not remove `generate_context!()` or weaken the
  Tauri bootstrap.
- Any tracked placeholder files under `dist/` -- `dist/` is ignored on purpose.
  Fix this in the build script, not by fighting the ignore rules.
- Frontend build output configuration -- this plan is only about making the
  backend test command runnable before the frontend has been built.

## Git workflow

- Branch: `advisor/012-bootstrap-frontend-dist-for-rust-tests`
- Commit message style: conventional commits, matching this repo's history
  (e.g. `fix(dx): create frontend dist before tauri context checks`)
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Ensure the expected frontend dist directory exists before Tauri validates it

In `src-tauri/build.rs`, create the workspace-root `dist/` directory before the
call to `tauri_build::build()`. Keep the inventory parsing and `TAURI_COMPILED_`
`COMMAND_ALLOWLIST` export exactly as they are.

The shape should be:

1. resolve the workspace root
2. create `workspace_root.join("dist")` with `fs::create_dir_all`
3. fail clearly if directory creation itself fails
4. continue with the existing inventory and `tauri_build::build()` flow

Do not add a checked-in placeholder file under `dist/`. The directory only
needs to exist for Tauri's bootstrap check; the real frontend build still owns
its own output during `npm run frontend:build`.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` -> exit 0.

### Step 2: Prove the full backend test command now runs from a clean checkout

Run the same command the README and future CI use:

`cargo test --manifest-path src-tauri/Cargo.toml`

It should now complete successfully on a checkout where `dist/` did not exist
before the build. If it still fails with the same Tauri panic, stop and report:
that means the directory bootstrap did not happen early enough and the fix must
be revisited rather than papered over.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml` -> exit 0.

## Test plan

- No new source-level test file is required.
- The regression is the full backend command itself, which should now pass on a
  clean checkout.
- Keep `cargo test --manifest-path src-tauri/Cargo.toml --lib` as the faster
  check during implementation, but use the full `cargo test` command as the
  final proof.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `src-tauri/build.rs` creates the frontend dist directory before Tauri
      validates `frontendDist`
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- The build script still cannot make the `frontendDist` path exist before
  `tauri_build::build()` or `generate_context!()` runs.
- The repository starts treating `dist/` as a tracked source directory for some
  other reason and the directory-bootstrap approach would conflict with that.
- `cargo test --manifest-path src-tauri/Cargo.toml` fails for a different,
  unrelated backend issue after this change. Do not start fixing that unrelated
  issue under this plan.

## Maintenance notes

- If the frontend output path changes later, this build-time bootstrap must be
  updated in lockstep with `tauri.conf.json`.
- Reviewers should confirm this fix does not introduce a committed placeholder
  artifact or a `.gitignore` exception. The directory is a build-time
  prerequisite, not source content.
