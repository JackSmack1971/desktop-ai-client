# Staging Document Schema

Use this schema for the root staging document. Keep headings and labels stable so downstream tooling can split the file deterministically.

## Required preamble

The file must begin with:

```markdown
<!-- generated-by: compiling-tribal-knowledge -->
# Repository Tribal Knowledge
```

Then include:

```markdown
## Capture Metadata

- Repository root: repository-relative dot (`.`)
- Git branch: exact current branch or `detached HEAD`
- Scope: full repository or the exact user-supplied scope
- Evidence cutoff: current local worktree and local Git history
- Generated file: `RAW_CONTEXT.md` or `STAGE_NOTES.md`
```

Do not include timestamps. Stable content improves diff quality and downstream caching.

## Required repository-wide sections

```markdown
## Evidence Rules
```

Define `[CONFIRMED]`, `[INFERRED]`, and `[UNRESOLVED]` exactly as used in the skill.

```markdown
## Cross-Cutting Contracts
```

Record only repository-wide rules that affect at least two major directories. Organize them under the same five dimensions when applicable. Omit empty subsections; do not invent content to fill them.

## Required directory section shape

Create one level-two heading per major directory using its exact repository-relative path:

```markdown
## src/example

### Core Ownership

[CONFIRMED] Atomic ownership statement.

Evidence: `src/example/index.ts` export surface; `tests/example-boundary.test.ts`.

### Architectural Invariants

[CONFIRMED] Atomic invariant statement.

Evidence: `src/example/service.ts::createService`; `tests/example-ordering.test.ts`.

### Historical Pitfalls

[UNRESOLVED] No reliable local history explains the retained compatibility branch.

Evidence: `src/example/legacy-adapter.ts`; local Git history is shallow.

### Stylistic Standards

[CONFIRMED] Atomic style or type-safety rule.

Evidence: `eslint.config.js`; `tsconfig.json`; `src/example/types.ts`.

### Hidden Contracts

[INFERRED] Atomic behavioral contract supported by two independent signals.

Evidence: `src/example/hydrate.ts::hydrateState`; `tests/example-restart.test.ts`.
```

Each of the five subsections is mandatory for every directory. A subsection may contain multiple atomic claims. Every claim cluster must be followed by at least one `Evidence:` line.

## Claim quality rules

- State one rule, boundary, pitfall, style discipline, or contract per paragraph.
- Prefer exact verbs: owns, rejects, emits, hydrates, persists, retries, invalidates, serializes, commits, rolls back, delegates.
- State out-of-scope responsibilities explicitly under Core Ownership.
- State ordering with numbered events or `A -> B -> C` when sequence is contractual.
- State cardinality, nullability, idempotency, retry limits, cache lifetime, or transaction boundaries when repository evidence defines them.
- Distinguish current behavior from compatibility residue.
- Name contradictory evidence under `[UNRESOLVED]`; never harmonize it silently.
- Do not paste source blocks longer than five lines. Cite paths and symbols instead.

## Exclusions

Do not include:

- General repository summaries.
- Dependency inventories without architectural meaning.
- Suggestions for future redesign.
- Generic best practices not evidenced by the repository.
- Secrets, credentials, environment values, personal information, or large log excerpts.
- Placeholder text, empty headings, TODO markers, or rhetorical questions.
