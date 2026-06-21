# Plan 004: Align the stop hook with the review output contract

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report - do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat aea7052..HEAD -- .claude/hooks/stop.js .codex/hooks/stop.js`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: MED
- **Depends on**: 005
- **Category**: dx
- **Planned at**: commit `aea7052`, 2026-06-21
- **Issue**: not published

## Why this matters

The stop hook is checking the wrong review contract. It still looks for generic `findings` / `evidence` / `remaining risk` headings, while the review workflow now expects `Verdict`, `Blocking findings`, `Non-blocking suggestions`, `Verification gaps`, and `Merge safety notes` and emits markdown headings for the same sections. As a result, valid review completions can be rejected and malformed completions can slip through.

## Current state

- Both hook copies contain the same parser logic and should stay in sync.
- `collectSections()` only recognizes `findings`, `evidence`, and `remaining risk`.
- `missingSections()` only requires those three keys.
- The review command contract now requires `Verdict`, `Blocking findings with file evidence`, `Non-blocking suggestions`, `Verification gaps`, and `Merge safety notes`.
- The workflow emits `## verdict`, `## blocking-findings`, `## non-blocking-suggestions`, `## verification-gaps`, and `## merge-safety-summary`.

Current code excerpts:

- `.codex/hooks/stop.js:24-28`
  - aliases for `findings`, `evidence`, and `remaining risk`
- `.codex/hooks/stop.js:68-92`
  - `missingSections()` checks those same three keys
  - `decision: "block"` is emitted without using the repo's exit-code convention
- `.codex/commands/review/pr.md:28-34`
  - required output sections for the review contract
- `.codex/workflows/review-readiness.js:747` and `:799`
  - emitted headings are `## verdict`, `## blocking-findings`, `## non-blocking-suggestions`, `## verification-gaps`, and `## merge-safety-summary`

Repo convention to follow:

- The hook-governance docs already use exit code `2` to block completion on validation failure. Prefer that convention over inventing a new JSON-only block signal.

## Commands you will need

| Purpose | Command | Expected on success |
|---------|---------|---------------------|
| Syntax check | `node --check .claude/hooks/stop.js` and `node --check .codex/hooks/stop.js` | exit 0 for both |
| Valid review sample | Pipe a markdown review with all required headings into each hook | exit 0 and no block decision |
| Invalid review sample | Pipe a markdown review missing one required heading into each hook | exit 2 and a clear reason on stderr |

## Scope

**In scope**

- `.claude/hooks/stop.js`
- `.codex/hooks/stop.js`

**Out of scope**

- `.codex/commands/review/pr.md`
- `.codex/workflows/review-readiness.js`
- any other hook files
- repo-wide docs unless the implementation proves the contract text itself is wrong

## Git workflow

- Branch: keep the current branch
- Commit: one mirrored hook fix
- Keep both copies identical unless the repository itself already diverges them for a reason

## Steps

### Step 1: Align the parser with the actual review headings

Change the hook to recognize the five real review sections instead of the old generic trio. Keep the `headingName()` normalization, but update the alias map and `missingSections()` to require the review contract's actual sections.

Prefer the workflow's emitted markdown headings as the source of truth. If the command doc and workflow still disagree after you inspect the live files, stop and report which one is authoritative instead of guessing.

**Verify**: `node --check .claude/hooks/stop.js` and `node --check .codex/hooks/stop.js` -> exit 0.

### Step 2: Make the block path use the repository's hook convention

Replace the JSON-only `decision: "block"` path with a real blocking failure that matches the repo's hook-governance convention. The clean behavior is:

- success path: emit the normal Stop hook JSON and exit 0
- block path: print the reason to stderr and exit 2

Keep the `stop_hook_active` escape hatch.

**Verify**: run a valid sample and an invalid sample through both hook files. The valid sample should exit 0, and the invalid sample should exit 2 with the missing section name in the reason.

## Test plan

- Use one valid sample review body that includes all five required headings.
- Use one invalid sample that omits exactly one required heading.
- Run both samples against both hook copies so the mirrored files cannot drift.

## Done criteria

- [ ] `node --check .claude/hooks/stop.js` exits 0
- [ ] `node --check .codex/hooks/stop.js` exits 0
- [ ] Valid review output passes through both hooks
- [ ] Invalid review output blocks with exit code 2 on both hooks
- [ ] No files outside the in-scope list are modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- The runtime clearly requires a JSON `decision` field and does not honor exit codes.
- The live review workflow output differs from the headings listed above.
- The two hook copies are no longer identical and the divergence has an intentional owner.

## Maintenance notes

- Keep the two hook copies identical unless the repo deliberately splits them later.
- A reviewer should check that the hook blocks only malformed review completions, not valid ones.
