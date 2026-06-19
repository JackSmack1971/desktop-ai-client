# Plan 007: Remove the duplicate npm lockfile and pin pnpm as the only package manager

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- package.json package-lock.json pnpm-lock.yaml`
> If any in-scope file changed since this plan was written, re-check whether
> `package-lock.json` still exists before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P3
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

The repo root has both `package-lock.json` and `pnpm-lock.yaml` checked in, but every script, doc, and CI assumption (`README.md`'s Quickstart, `pnpm-workspace.yaml`'s presence) assumes pnpm is the only package manager used. Two lockfiles can drift independently — whichever tool a contributor or future CI step runs last silently becomes authoritative for `node_modules` resolution, with no guarantee it matches what was actually tested with the other tool. This is low risk today given the small dependency tree, but it's a cheap, mechanical fix to remove before it compounds.

## Current state

- Confirmed present at repo root: `package-lock.json` (62,054 bytes) and `pnpm-lock.yaml` (40,577 bytes).
- `package.json` has no `packageManager` field.
- `README.md`'s Quickstart section already only documents `pnpm install --frozen-lockfile` — no npm instructions anywhere in the docs.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Confirm pnpm still resolves cleanly | `pnpm install --frozen-lockfile` | exit 0 |
| Type-check (sanity) | `pnpm check` | exit 0 |

## Scope

**In scope**:
- Delete `package-lock.json`.
- Add a `packageManager` field to `package.json`.

**Out of scope**:
- Any change to `pnpm-lock.yaml` itself.
- Adding a `preinstall` guard script that blocks plain `npm install` — a reasonable follow-up, but adds a new script/dependency-on-behavior beyond this plan's "delete the stray file" scope; mention it in your final report as an optional follow-up, don't implement it here.

## Git workflow

- Branch: `advisor/007-remove-duplicate-npm-lockfile`
- Commit message: `chore: remove stray package-lock.json, pin pnpm via packageManager`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Determine the pnpm version to pin

Run `pnpm --version` and note the output (this repo's README states "pnpm 8+" as the prerequisite — use whatever version is actually installed in your environment, or `8.0.0` if you want to pin the documented minimum instead of your local version; prefer pinning the locally-resolved version since that's what actually produced `pnpm-lock.yaml`'s lockfile format).

### Step 2: Add `packageManager` to `package.json`

Add a `"packageManager"` field to `package.json`, placed after `"private": true,` (matching the existing key ordering style — `name`, `version`, `private`, `type`, then this new field, then `scripts`):

```json
"packageManager": "pnpm@<version-from-step-1>",
```

### Step 3: Delete `package-lock.json`

Delete the file at repo root: `package-lock.json`.

**Verify**: `pnpm install --frozen-lockfile` → exit 0 (confirms `pnpm-lock.yaml` alone is still sufficient and unaffected by the deletion). `pnpm check` → exit 0.

## Test plan

No new test needed — this is a manifest/lockfile change with no executable behavior change. The verification command in Step 3 is the regression guard: if `pnpm-lock.yaml` were somehow out of sync with `package.json` (it isn't, per recon), `--frozen-lockfile` would fail and surface that immediately.

## Done criteria

- [ ] `package-lock.json` no longer exists at repo root
- [ ] `package.json` has a `"packageManager": "pnpm@..."` field
- [ ] `pnpm install --frozen-lockfile` exits 0
- [ ] `pnpm check` exits 0
- [ ] No files outside `package.json`/`package-lock.json` are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

- `pnpm install --frozen-lockfile` fails after deleting `package-lock.json` — this would mean something in the toolchain actually depends on the npm lockfile (unexpected, but if it happens, restore the file and report rather than forcing the install with `--no-frozen-lockfile`).

## Maintenance notes

- If a contributor reports `npm install` "not working" after this lands, that's expected and correct — point them at the pnpm Quickstart in `README.md`.
- Consider the `preinstall` guard script (e.g. the `only-allow` package, or a one-line `if (!process.env.npm_execpath?.includes('pnpm'))` check) as a future hardening step if a non-pnpm install ever actually happens in practice — not needed preemptively.
