# CONTRIBUTING.md Blueprint

Build a repository-specific document from this sequence. Omit unsupported sections instead of filling them with generic advice.

## 1. Title and welcome

Use an H1 in the form `# Contributing to` followed by the repository's actual project name. Thank contributors, identify the project's purpose in one sentence, and name accepted contribution categories supported by the repository.

## 2. Before contributing

Include only verified prerequisites:

- Code of Conduct or community standards.
- License or contribution agreement implications.
- Supported runtime, package manager, compiler, container, or operating-system requirements.
- Security-reporting warning before public issue instructions.

## 3. Find or propose work

Explain the actual path for:

- Existing issues and newcomer labels.
- Bug reports and required reproductions.
- Feature proposals and design discussion.
- Questions or support.
- Documentation, tests, translations, design, or triage.

Do not mention Discussions, labels, forms, or forums that do not exist.

## 4. Repository scope

Describe major contributor-relevant areas and protected boundaries. Include generated files, vendored code, lockfiles, migrations, snapshots, fixtures, schemas, public APIs, or release files only when the repository establishes special handling.

## 5. Local setup

Provide a linear, copy-ready path:

1. Fork/clone and upstream remote instructions when appropriate for the repository's collaboration model.
2. Runtime and system dependencies with exact versions or version sources.
3. Dependency installation using the repository's selected package manager.
4. Environment configuration using example files, local services, containers, or bootstrap tasks.
5. A smoke test proving setup succeeded.

Keep secrets out of examples. Never instruct contributors to copy real credential files.

## 6. Development workflow

Explain the verified default branch and branch workflow. Add branch naming only when required or consistently documented. Include package or service selection for monorepos.

## 7. Quality standards

Translate each standard into an action and a check:

- Formatting and linting.
- Type safety or static analysis.
- Unit, integration, end-to-end, snapshot, property, mutation, or documentation tests.
- Public API documentation, changelog, migration, schema, accessibility, localization, or compatibility requirements.
- Generated output refresh rules.

Provide targeted commands before full-suite commands when both are supported.

## 8. Commit guidance

Include a commit format only when policy, tooling, templates, or consistent history supports it. Distinguish enforced requirements from examples. Mention sign-off or signature requirements only when configured.

## 9. Pull requests

State:

- Correct base branch.
- Required issue/design linkage.
- Scope and reviewability expectations.
- Tests, docs, screenshots, migration notes, release notes, or performance evidence required by templates or CI.
- Draft pull-request usage when the repository supports it.
- Review ownership and update/rebase expectations when documented.

End with a pre-flight checklist that mirrors the actual merge gates.

## 10. Security, legal, and recognition

Link to the repository's private security process. Summarize license, DCO, CLA, citation, or recognition mechanisms only when present. Do not turn descriptive license text into legal advice.

## 11. Getting help

Name only verified contact channels. Prefer a repository-maintained support file, issue form, discussion category, chat link, mailing list, or maintainer route. Do not invent response-time expectations.

## Presentation rules

- Use GitHub alerts sparingly for genuine note, tip, important, warning, or caution content.
- Keep headings predictable and sentences direct.
- Prefer short paragraphs, numbered procedures, and small decision tables over walls of prose.
- Use repository-relative links for checked-in files.
- Avoid duplicating the README; CONTRIBUTING.md should connect setup to contribution-specific execution.
- Keep the file maintainable: every command should have an obvious source of truth.
