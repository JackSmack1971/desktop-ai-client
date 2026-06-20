# Lifecycle Hook Guidance

The skill must behave as though the following controls are enforced. Project owners may implement equivalent hooks in `.claude/settings.json`; do not install or alter hooks during this skill.

## PreToolUse

For `Write`, `Edit`, and command execution:

- Permit persistent writes only to the repository-root `CONTRIBUTING.md`.
- Reject writes to `.env*`, key material, credential stores, Git internals, dependency directories, generated build output, and files outside the repository root.
- Reject destructive Git operations, package installation, lockfile mutation, network retrieval, release commands, migrations, deployment, staging, and commits.
- Require path normalization before comparing a requested path with the allowed target.

## PostToolUse

After `Write` or `Edit` of `CONTRIBUTING.md`:

- Run the bundled validator.
- Run `git diff --check -- CONTRIBUTING.md`.
- Summarize changed headings and validator errors without printing unrelated repository content.

## Stop and TaskCompleted

Block successful completion unless:

- The root file exists and is non-empty.
- The validator exits zero.
- `git diff --check` exits zero.
- No unresolved placeholders remain.
- The skill created no persistent file other than `CONTRIBUTING.md`.
- No file was staged or committed.

## SubagentStop

The forked agent must return a compact mailbox handoff containing:

- Output path.
- Repository archetype.
- Authoritative evidence files.
- Documented commands and merge gates.
- Omitted unsupported policies.
- Validation results and unresolved warnings.

Do not return raw inventory output, full file contents, command logs, or private repository data unless the user explicitly requests them.
