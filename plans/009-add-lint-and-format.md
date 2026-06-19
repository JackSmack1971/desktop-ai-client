# Plan 009: Add ESLint + Prettier with `lint`/`format` scripts

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- package.json`
> If `package.json` changed since this plan was written, re-read its current
> `scripts`/`devDependencies` before proceeding; on a conflicting addition
> (e.g. someone already added a `lint` script), treat it as a STOP condition.

## Status

- **Priority**: P3
- **Effort**: S-M
- **Risk**: LOW
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

There is no lint or format tooling anywhere in this repo — no `.eslintrc*`/`eslint.config.*`, no `.prettierrc*`, no `.editorconfig`, and no `lint`/`format` script in `package.json`. `pnpm check` (via `svelte-check`) catches type errors but not unused variables, accessibility issues in `.svelte` markup, or style drift across contributors (human or AI-generated diffs). Adding baseline tooling now, while the codebase is small (~24 frontend files), is far cheaper than retrofitting it after the file count grows.

## Current state

- `package.json` scripts (lines 6-14): `dev`, `build`, `preview`, `check`, `check:watch`, `frontend:dev`, `frontend:build`, `tauri`.
- `package.json` `devDependencies` (lines 16-26): `@sveltejs/adapter-static ^3.0.0`, `@sveltejs/kit ^2.0.0`, `@sveltejs/vite-plugin-svelte ^5.0.0`, `@tauri-apps/cli ^2.0.0`, `@types/node ^25.9.3`, `svelte ^5.0.0`, `svelte-check ^4.0.0`, `typescript ^5.0.0`, `vite ^6.0.0`.
- Confirmed via exhaustive glob search: zero `.eslintrc*`, `eslint.config.*`, `.prettierrc*`, `prettier.config.*`, or `.editorconfig` files anywhere in the repo (root or `src-tauri/`).
- `.claude/rules/frontend.md` already documents a detailed set of Svelte 5 / TypeScript conventions this codebase is expected to follow (runes usage, `$derived` vs `$effect`, typed props, etc.) — the ESLint config this plan adds should lean on `eslint-plugin-svelte` and `@typescript-eslint` to catch what's mechanically catchable from those rules (e.g. unused `$state`, missing `lang="ts"`), not attempt to encode every prose rule as a custom lint rule.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Install deps | `pnpm install` | exit 0 |
| Run lint | `pnpm lint` | exit 0, or a finite list of pre-existing warnings (see Step 4) |
| Run format check | `pnpm format` | exit 0 |
| Typecheck (regression guard) | `pnpm check` | exit 0, unaffected by this plan |

## Scope

**In scope**:
- `package.json` — add `devDependencies` and `lint`/`format` scripts.
- New file: `eslint.config.js` (flat config — required for current ESLint major versions).
- New file: `.prettierrc.json`
- New file: `.editorconfig`

**Out of scope**:
- Auto-fixing every lint warning that surfaces across the existing 24 frontend files — see Step 4's handling of pre-existing findings. Do not silently rewrite working code beyond what `--fix` safely auto-corrects.
- Linting `src-tauri/**/*.rs` — Rust already has `cargo check`/`clippy` as its native equivalent; if the team wants `clippy` wired in, that's better scoped as part of `plans/008-add-ci-workflow.md`'s backend job, not this frontend-tooling plan. Do not add a Rust linter here.
- A pre-commit hook (e.g. husky/lint-staged) — a reasonable follow-up, but adds new tooling/dependencies beyond "add lint and format configs"; mention it as an optional next step in your final report.

## Git workflow

- Branch: `advisor/009-add-lint-and-format`
- Commit message: `chore: add eslint and prettier`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Install dependencies

```
pnpm add -D eslint eslint-plugin-svelte @typescript-eslint/eslint-plugin @typescript-eslint/parser prettier prettier-plugin-svelte
```

**Verify**: `pnpm install` → exit 0; `package.json`'s `devDependencies` now includes these 6 packages.

### Step 2: Add `eslint.config.js` (flat config)

```js
import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';

export default [
	js.configs.recommended,
	...tseslint.configs.recommended,
	...svelte.configs['flat/recommended'],
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parserOptions: {
				parser: tseslint.parser,
			},
		},
	},
	{
		ignores: ['build/', '.svelte-kit/', 'dist/', 'src-tauri/target/', 'node_modules/'],
	},
];
```

If `typescript-eslint` (the unified meta-package, distinct from `@typescript-eslint/eslint-plugin` + `@typescript-eslint/parser` installed separately in Step 1) is preferred for simpler flat-config setup, install it instead: `pnpm add -D typescript-eslint` and use `tseslint.configs.recommended` as shown — check `eslint-plugin-svelte`'s own setup docs for the currently-recommended flat-config pattern, since flat-config conventions for Svelte have changed across versions; match whatever the installed versions' own documentation shows rather than this snippet verbatim if they've diverged.

**Verify**: `pnpm dlx eslint --print-config src/lib/stores/chat.ts` → prints a resolved config with no errors (confirms the config file itself is syntactically valid and loadable).

### Step 3: Add `.prettierrc.json` and `.editorconfig`

`.prettierrc.json`:
```json
{
	"useTabs": true,
	"singleQuote": true,
	"plugins": ["prettier-plugin-svelte"],
	"overrides": [{ "files": "*.svelte", "options": { "parser": "svelte" } }]
}
```
(`useTabs: true` matches the existing indentation style observed in `src/lib/stores/chat.ts` and `src/lib/components/chat/ChatInput.svelte` during recon — both use tabs. Confirm this against a couple more existing files before locking it in; if the codebase is actually mixed, default to tabs since that's what the two files read during this audit use, and let Prettier's first run normalize the rest.)

`.editorconfig`:
```ini
root = true

[*]
indent_style = tab
charset = utf-8
trim_trailing_whitespace = true
insert_final_newline = true

[*.{json,yml,yaml,md}]
indent_style = space
indent_size = 2
```

### Step 4: Add `lint` and `format` scripts, run them, triage pre-existing findings

Add to `package.json`'s `scripts`:
```json
"lint": "eslint .",
"format": "prettier --check .",
"format:write": "prettier --write ."
```

Run `pnpm lint`. If it reports findings in pre-existing files (likely, since no linter has ever run on this codebase), do **not** mass-fix everything blindly:
1. Run `pnpm dlx eslint . --fix` to apply only the auto-fixable subset (typically formatting-adjacent rules ESLint marks as safely fixable).
2. Re-run `pnpm lint` and read what remains.
3. For each remaining finding, fix it only if it's a clear, low-risk correction (e.g. an unused import, a missing `lang="ts"` ESLint can't auto-add but is a one-line fix). If a finding looks like it requires understanding intent you don't have (e.g. a suspicious-looking but possibly-intentional pattern), leave it and list it in your final report as "pre-existing lint debt, not fixed under this plan" rather than guessing.

Run `pnpm format:write` once to normalize existing files to the new Prettier config, then `pnpm format` to confirm it now passes clean.

**Verify**: `pnpm lint` → exit 0 (after the triage above) or a short, explicitly-documented list of intentionally-deferred findings. `pnpm format` → exit 0.

## Test plan

No new application test needed — this plan adds tooling, not application behavior. The regression guard is Step 4's final `pnpm check` run (confirm Prettier's reformatting didn't change any code semantics, only whitespace/quote style):

**Verify**: `pnpm check` → exit 0, identical to its pre-plan result.

## Done criteria

- [ ] `pnpm lint` exits 0 (or documents specific deferred pre-existing findings in the final report)
- [ ] `pnpm format` exits 0
- [ ] `pnpm check` still exits 0 after Prettier's reformatting
- [ ] `eslint.config.js`, `.prettierrc.json`, `.editorconfig` exist at repo root
- [ ] `package.json` has `lint`, `format`, `format:write` scripts
- [ ] No `.svelte`/`.ts` file's *behavior* changed — only formatting/lint-fix-level edits (verify with `git diff` review of any file Prettier or `eslint --fix` touched beyond whitespace/quotes)
- [ ] `plans/README.md` status row updated

## STOP conditions

- `eslint --fix` or `prettier --write` produces a diff that changes more than whitespace/quote-style/import-ordering in any file (e.g. it reorders logic or changes a runtime value) — stop and inspect that specific file manually rather than trusting the auto-fixer blindly.
- A lint rule conflicts directly with an explicit, documented convention in `.claude/rules/frontend.md` (e.g. if a recommended rule discourages a pattern the rules file mandates) — do not silently disable the rule; report the conflict so a human can decide whether to adjust the ESLint config or the rules doc.

## Maintenance notes

- A pre-commit hook (husky + lint-staged) was considered but deliberately not added here — flagged as a follow-up if lint/format drift becomes a recurring problem in PRs.
- Once `plans/008-add-ci-workflow.md` lands (or is re-visited after this plan), add a `pnpm lint` and `pnpm format` step to its `frontend` job.
