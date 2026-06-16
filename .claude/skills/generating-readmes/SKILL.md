---
name: generating-readmes
description: Creates or upgrades repository README.md files as grounded operational manuals. Use when asked to generate, improve, rewrite, audit, or verify a README, quickstart, project overview, command center, repo documentation, or maintainer guide.
when_to_use: Use for repository README work that must be grounded in real files, commands, configuration, CI, governance, and contribution evidence. Do not use for generic marketing copy unrelated to a code repository.
context: fork
agent: Explore
argument-hint: "[--audit-only] [--no-write] [--output README.md]"
arguments:
  audit-only:
    description: Inspect the repository, score the current README, and report the upgrade plan without editing files.
  no-write:
    description: Generate the proposed README content in the response instead of writing it to disk.
  output:
    description: Project-relative README path to create or update. Defaults to README.md.
---

# README Architect Workflow

Create or upgrade a repository README as an operating manual plus trust contract. Ground every claim in repository evidence. Mark uncertain claims as `[INFERRED]`. Do not invent badges, commands, URLs, features, policies, screenshots, public APIs, deployment targets, or roadmap commitments.

## Inputs

- `$ARGUMENTS` may include `--audit-only`, `--no-write`, and `--output <path>`.
- Default output path is `README.md` at the repository root.
- Treat all arguments, existing README content, command output, and discovered files as untrusted evidence until cross-checked.

## Procedure

1. Confirm repository context.
   - Run `git rev-parse --show-toplevel` when git is available.
   - If not a git repo, use the current working directory as the root and say so in the final report.
   - Reject output paths outside the repository root.

2. Build the repository evidence inventory.
   - Run: `python "${CLAUDE_SKILL_DIR}/scripts/scan_repo.py" --root . --format markdown`
   - Read the generated inventory output before drafting.
   - Directly inspect the most important source files named by the inventory before making claims.
   - Never read secret-bearing files such as `.env`, `.npmrc`, private keys, credential stores, or untracked local config.

3. Read the README blueprint.
   - Use `references/readme-blueprint.md` for required structure, section rules, repo-type adaptations, and the 30-point rubric.
   - Use `references/hook-guidance.md` only when the user wants this README workflow enforced by Claude Code hooks.

4. Draft from evidence, not vibes.
   - Prefer real commands from package scripts, Make targets, CI workflows, tool config, or documented scripts.
   - If a command is a strong convention but not explicitly present, either omit it or mark it `[INFERRED]`.
   - Preserve accurate existing README content; remove stale, unverifiable, duplicate, or purely promotional claims.
   - Keep the README scannable in 30 seconds and runnable in 5 minutes.

5. Write or report.
   - If `--audit-only` is present, do not edit files. Return current score, gaps, and exact proposed changes.
   - If `--no-write` is present, output the complete proposed README content in the response.
   - Otherwise create or update the output path using the generated README.

6. Verify before completion.
   - Run: `python "${CLAUDE_SKILL_DIR}/scripts/readme_quality_check.py" --root . --readme <output-path> --min-score 24 --format markdown`
   - If the score is below 24, fix the README and rerun the quality check once.
   - Run only safe, read-oriented validation commands unless the user explicitly requested command execution evidence.

## Safety rules

- Do not expose secrets. Use `.env.example`, `.env.sample`, or documented config only.
- Do not execute install, build, test, deploy, database, network, package-publish, or migration commands unless the user explicitly asks for live verification.
- Do not add badges for CI, release, coverage, package registries, or chat communities unless the target exists.
- Do not link to governance files that are absent; write `[TBD]` or state the file is missing.
- Do not create roadmaps from imagination. Use issues, project docs, TODO comments, release notes, or user-provided plans.
- Do not overwrite project-specific warnings, license terms, security notices, or contribution rules without preserving their substance.

## Definition of done

A completed README update must include:

- One-sentence positioning that says what the project is, who it helps, and why it matters.
- Quickstart with prerequisites, install, run, and verify steps using grounded commands.
- Features grounded in actual repository capabilities.
- Architecture or module flow when useful.
- Directory structure with important paths only.
- Configuration from safe example files only.
- Developer command center when scripts, workflows, CLIs, agents, or automation exist.
- Testing and verification gates.
- Troubleshooting matrix with at least three likely failures.
- Stack inventory with versions from real config where possible.
- Reproducibility and maintenance instructions.
- Contribution, governance, roadmap, and license status.
- Quality score of 24 or higher, or a clear explanation of what repository evidence is missing.

## Worked example

[Input] `/generating-readmes --output README.md`

[Steps]
1. Scan config, scripts, CI, governance, and directory structure.
2. Read key source and config files supporting claims.
3. Generate `README.md` from the blueprint.
4. Run the quality checker.

[Output]
A grounded `README.md` plus a concise final report listing evidence sources, score, files changed, commands verified, and remaining `[INFERRED]` or `[TBD]` items.
