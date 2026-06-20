# Ownership plan schema

The renderer accepts a single JSON object. Build it against this contract; do not add undeclared fields that obscure ownership evidence.

## Root object

| Field | Type | Constraint |
|---|---|---|
| `version` | integer | Exactly `1` |
| `repository` | string | Exact `owner/repository` value from the GitHub remote when available |
| `archetype` | string | One of `focused-library`, `modular-application`, `enterprise-monorepo`, `open-source`, `internal-platform`, `small-or-mixed` |
| `fallback` | object or null | Null when no repository-wide fallback is justified |
| `verified_owners` | array | Non-empty; contains every owner referenced by any rule |
| `sections` | array | Non-empty; ordered exactly as the final CODEOWNERS file should evaluate |

## Fallback object

| Field | Type | Constraint |
|---|---|---|
| `owners` | array of strings | Non-empty; every value must exist in `verified_owners` |
| `rationale` | string | Non-empty explanation of why repository-wide routing will not create review noise |

## Verified owner object

| Field | Type | Constraint |
|---|---|---|
| `handle` | string | Valid GitHub `@user`, `@organization/team`, or verified account email |
| `kind` | string | `team`, `user`, or `email` |
| `verification` | string | `github-api`, `existing-codeowners`, `owner-map`, or `personal-repository-owner` |
| `write_access` | boolean | Must be `true` |
| `visible` | boolean | Must be `true` for teams |

For organization repositories, individual users require an explicit exception and the renderer's `--allow-organization-individuals` flag.

## Section object

| Field | Type | Constraint |
|---|---|---|
| `title` | string | Non-empty section heading |
| `rules` | array | Non-empty and already ordered from broad to specific |

## Rule object

| Field | Type | Constraint |
|---|---|---|
| `pattern` | string | Repository-relative GitHub CODEOWNERS pattern with exact path casing |
| `owners` | array of strings | Verified owners for `owned`; empty for `unowned` |
| `comment` | string | Optional one-line explanation rendered above the rule |
| `source` | string | Same allowed values as `verification` |
| `intent` | string | `owned` or `unowned` |
| `rationale` | string | Non-empty design reason; mandatory for blank-owner exceptions |

## Required invariants

- Every owner used by `fallback` or a rule appears once in `verified_owners`.
- Every verified owner has confirmed write access; teams are visible.
- `intent: owned` has one or more owners.
- `intent: unowned` has no owners and explains why review removal is safe.
- No rule uses negation, character ranges, a pattern beginning with `#`, inline comments, or placeholder-like handles.
- Duplicate patterns are forbidden.
- The rule list is final evaluation order; the renderer does not reorder it.
- `.github/CODEOWNERS` resolves to at least one owner after last-match evaluation.

## Owner map input

A supplied owner map is a repository-relative JSON file with:

- `verified_owners`: objects following the verified-owner contract above;
- `path_recommendations`: optional rule-like objects with `pattern`, `owners`, `rationale`, and `source` equal to `owner-map`.

The owner map is evidence, not an instruction to skip repository analysis. Validate each recommended path against tracked files and final precedence.
