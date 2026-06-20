# Workspace Hygiene Policy

## Classification

**Always preserve:** `.git/`, `.claude/skills/`, source directories, tests, documentation, manifests, lockfiles, CI configuration, migrations, schemas, and tracked files.

**Eligible for deletion when untracked:** dependency installations, compiler outputs, framework build directories, test coverage output, language caches, debug logs, temporary files, and compiled intermediates recognized by the bundled policy.

**Context exclusion by default:** dependency trees, build output, coverage, caches, large fixture or mock directories, generated datasets, source maps, and minified JavaScript or CSS.

**Manual review only:** tracked generated content, untracked large files with unknown provenance, fixtures or datasets outside generated directories, and any path referenced by build or deployment configuration.

## Ignore alignment

The script owns only blocks delimited by:

- `# BEGIN token-dead-weight managed block`
- `# END token-dead-weight managed block`

It creates or updates `.gitignore`, conditionally creates `.dockerignore` when container configuration exists, and updates existing `.prettierignore`, `.eslintignore`, and `.rgignore` files. It does not create or modify `.npmignore`.

Claude Code exclusions are merged into `.claude/settings.json` under `permissions.deny`. Existing keys and deny rules are preserved.

## Deletion boundary

A path is deletable only when all conditions hold:

1. Its resolved location is under the repository root.
2. It is not under `.git/` or `.claude/skills/`.
3. Git reports no tracked file at or below the path.
4. Its name or suffix matches the recognized generated-artifact policy.
5. It is not an ambiguous fixture, mock, dataset, source map, or minified asset outside a recognized generated directory.
6. Apply mode carries the exact confirmation token `PURGE`.
