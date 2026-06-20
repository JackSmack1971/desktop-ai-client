---
name: generate-codeowners
description: Analyzes a Git repository's architecture, history, risk boundaries, existing governance, and verified GitHub ownership data to generate or audit an optimal GitHub CODEOWNERS file. Use manually when establishing or revising repository code ownership; it never invents GitHub handles or writes unresolved owners.
when_to_use: Use for CODEOWNERS creation, ownership-map redesign, monorepo ownership routing, or review bottleneck remediation. Do not use for GitLab approval sections or Bitbucket-specific ownership syntax.
compatibility: Requires Git, Bash, and Python 3.10 or later available as python3 or python. GitHub CLI is optional and used only for read-only owner verification.
disable-model-invocation: true
argument-hint: "[generate|audit] [owner-map.json]"
arguments:
  - mode
  - owner_map
context: fork
agent: general-purpose
---

# Generate CODEOWNERS

Create or audit GitHub CODEOWNERS from repository evidence. Treat `$mode` as `generate` when omitted. Treat `$owner_map` as optional. Only accept `generate` or `audit`.

## Non-negotiable rules

- Never invent, guess, or synthesize GitHub usernames, organization names, or team slugs.
- Prefer visible GitHub teams with explicit write access. Use an individual only for a personal repository or when the supplied owner map explicitly authorizes that exception.
- Do not change repository rulesets, branch protection, team membership, permissions, or bot bypass settings.
- Write only `.github/CODEOWNERS`; GitHub checks `.github/` before root and `docs/`.
- Preserve repository path casing exactly. Order rules from broad to specific because the last matching rule wins.
- Keep the file under 3 MiB. Put comments on separate lines. Do not use `!`, character ranges, or an escaped leading `#` pattern.
- Use blank-owner rules only for an explicitly documented generated or low-risk path. Never use them to bypass review on security, CI/CD, release, dependency, infrastructure, authentication, authorization, cryptography, database schema, or policy files.
- Protect `.github/CODEOWNERS` itself with a verified owner.

## Inputs

1. `mode`: `generate` or `audit`; default `generate`.
2. `owner_map`: optional project-relative JSON file following `references/ownership-plan-schema.md`. Reject absolute paths, symlinks escaping the repository, and files outside the repository root.
3. Repository evidence: tracked files, manifests, architecture, Git history, existing ownership files, governance documents, and read-only GitHub metadata when `gh` is installed and authenticated.

## Procedure

1. Resolve the repository root with `git rev-parse --show-toplevel`. Stop with a precise error outside a Git worktree.
2. Read `references/design-policy.md`. Read `references/ownership-plan-schema.md` before creating a plan.
3. Create an untracked state directory using `git rev-parse --git-path claude-codeowners`; do not place analysis artifacts in the working tree.
4. Run:

   ```bash
   "${CLAUDE_SKILL_DIR}/scripts/run_python.sh" "${CLAUDE_SKILL_DIR}/scripts/analyze_repository.py" \
     --repo "$(git rev-parse --show-toplevel)" \
     --output "$(git rev-parse --git-path claude-codeowners)/inventory.json"
   ```

5. Run the read-only GitHub discovery script. A missing CLI, missing authentication, or insufficient API scope is a recoverable condition, not permission to invent owners.

   ```bash
   "${CLAUDE_SKILL_DIR}/scripts/run_python.sh" "${CLAUDE_SKILL_DIR}/scripts/discover_github_owners.py" \
     --repo "$(git rev-parse --show-toplevel)" \
     --output "$(git rev-parse --git-path claude-codeowners)/github-owners.json"
   ```

6. Read the active CODEOWNERS file and the governance files identified by the inventory. Existing handles are evidence only until their current access is verified.
7. In `audit` mode:
   - Locate the active CODEOWNERS file using GitHub precedence: `.github/CODEOWNERS`, `CODEOWNERS`, then `docs/CODEOWNERS`.
   - Run `validate_codeowners.py` against that file.
   - Report parser errors, dead rules, shadowing, unowned paths, individual-owner risks, missing self-ownership, and architectural coverage gaps.
   - Do not modify files.
8. In `generate` mode, classify the repository before designing rules:
   - focused library or SDK;
   - modular application;
   - enterprise monorepo;
   - open-source project;
   - internal platform or infrastructure repository;
   - small or mixed repository when evidence is insufficient for a stronger classification.
9. Resolve owners using this evidence order:
   1. verified entries in the supplied owner map;
   2. valid teams already present in the active CODEOWNERS file;
   3. visible repository teams returned by GitHub with `push`, `maintain`, or `admin` permission;
   4. the verified repository owner for a personal repository.
10. Do not infer GitHub handles from commit author names or email addresses. Use Git history only to identify domain concentration, review-load risk, abandoned areas, and likely ownership boundaries.
11. Design the ownership map:
    - Map stream-aligned product domains to their own teams.
    - Map root build configuration, shared tooling, CI/CD, release, and infrastructure to platform or repository-steward teams.
    - Add dual-team ownership only where two independent knowledge domains are genuinely required; remember that GitHub accepts approval from any listed owner, not all listed owners.
    - Keep unit tests with their product domain. Give integration or system test suites a separate QA owner only when the repository structure supports it.
    - Use a fallback only when the repository archetype and verified team capacity justify the notification load.
    - Add increasingly specific overrides after broad rules.
    - Prefer directory rules over broad extension rules. A late `*.js`, `*.yaml`, or similar rule can silently override domain ownership.
12. If any required production domain lacks a verified owner, do not write `.github/CODEOWNERS`. Produce a concise ownership-gap report listing exact paths and the owner roles that must be mapped.
13. Create `ownership-plan.json` in the state directory. Every owner used by a rule must appear in `verified_owners`; every blank-owner rule must use `intent: unowned` and include a non-empty rationale.
14. Render atomically:

    ```bash
    "${CLAUDE_SKILL_DIR}/scripts/run_python.sh" "${CLAUDE_SKILL_DIR}/scripts/render_codeowners.py" \
      --repo "$(git rev-parse --show-toplevel)" \
      --plan "$(git rev-parse --git-path claude-codeowners)/ownership-plan.json"
    ```

15. Validate immediately:

    ```bash
    "${CLAUDE_SKILL_DIR}/scripts/run_python.sh" "${CLAUDE_SKILL_DIR}/scripts/validate_codeowners.py" \
      --repo "$(git rev-parse --show-toplevel)" \
      --file ".github/CODEOWNERS" \
      --json-out "$(git rev-parse --git-path claude-codeowners)/validation.json"
    ```

16. Fix and revalidate until the validator exits zero. Then inspect `git diff -- .github/CODEOWNERS` and confirm no unrelated tracked file changed.

## Completion contract

Return:

- repository archetype and evidence;
- generated or audited CODEOWNERS path;
- owner-resolution sources and any user-asserted entries;
- coverage summary, intentional blank-owner paths, and high-risk dual-owner paths;
- validator result and exact unresolved warnings;
- ruleset recommendations as advisory text only.

Do not claim success unless the target file exists in generate mode, validation exits zero, `.github/CODEOWNERS` is self-owned, and the working-tree diff is limited to the expected file.

## Command governance

- `PreToolUse`: block destructive cleanup, history rewriting, permission changes, and writes outside `.github/CODEOWNERS` plus the Git state directory.
- `PostToolUse`: rerun validation after the target file changes.
- `Stop` or `TaskCompleted`: require a zero-error validation result and an expected-only diff.
- `SubagentStop`: return the generated path, validation JSON path, coverage counts, and unresolved warnings.

Use the session-scoped hook profile in `references/hook-guidance.md` when deterministic enforcement is required.

## Troubleshooting

- **No verified owners:** supply a repository-relative owner map or authenticate `gh` with repository and organization read access. Do not weaken verification.
- **Team is returned but unusable:** confirm the team is visible and has explicit write-level repository access; direct permissions on individual members are insufficient for reliable team routing.
- **Validator reports shadowing:** move broad patterns earlier or replace late extension globs with domain-specific paths.
- **Large unowned set:** add a justified fallback or explicit domain rules; do not hide the gap with blank-owner exceptions.
- **Generated or vendored files dominate reviews:** add a narrowly scoped blank-owner override only after verifying the path is reproducible and protected against manual edits elsewhere.

## Worked example

**[Input]** `generate` mode; repository inventory shows `services/payments/`, `.github/workflows/`, and `db/migrations/`; GitHub discovery verifies product, platform, and database teams with write access.

**[Steps]** Classify as a modular application; place a justified fallback first only when a verified steward team exists; assign product ownership to the service; assign platform ownership to workflows; assign database plus product ownership to migrations; add `.github/CODEOWNERS` self-ownership; validate last-match behavior and coverage.

**[Output]** `.github/CODEOWNERS`, an untracked inventory and validation report under the Git metadata directory, and a final summary identifying ownership sources, exceptions, and ruleset recommendations.
