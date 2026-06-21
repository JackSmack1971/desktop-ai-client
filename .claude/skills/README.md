# Skills

Skills isolate repeatable procedures that would otherwise bloat `CLAUDE.md`.

## Skill Stack

- `stack-detection`: classify the repo before architectural or client-layer changes
- `repo-audit`: compose the focused audit skills into one evidence-led review
- `privacy-boundary-review`: inspect secrets, file intake, telemetry, and renderer boundaries
- `provider-routing-review`: inspect provider selection, fallback, and streaming transport
- `storage-recovery-review`: inspect SQLite, migrations, FTS, retention, and corruption recovery
- `release-evidence-review`: inspect release inventory, command exposure, and evidence completeness
- `generate-codeowners`: generate or audit `CODEOWNERS` from verified repository evidence
- `generating-readmes`: create or upgrade repository READMEs from grounded evidence
- `generating-contributing-guidelines`: build contribution guidance from repository structure
- `compiling-tribal-knowledge`: capture staged notes into a reusable knowledge artifact
- `pr-triage`: triage incoming PRs and route them to the right follow-up
- `optimize-github-operations`: tighten GitHub workflows, templates, and hooks
- `purging-token-dead-weight`: remove redundant token-heavy content from the workspace
- `route-to-agent`: choose the right subagent for the changed paths

## Suggested Composition

1. `stack-detection`
2. one or more focused audit skills
3. `repo-audit` for broad synthesis
