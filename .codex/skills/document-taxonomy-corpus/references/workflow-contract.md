# Workflow Contract

## Category Landmarks

### ADR

- Durable decision record
- Context, problem, considered options, decision, consequences, confirmation
- Decision-makers, consulted, informed, status, date

### PRD

- Product intent and desired behavior
- Objectives, personas, success metrics, scope, stories, requirements, dependencies, timeline, open questions
- Living document for product, design, and engineering stakeholders

### SPEC_RFC

- Technical blueprint or request for comments
- Architecture, components, APIs, schemas, flows, rollout, testing, failure modes, alternatives
- Implementation detail dominates product intent

## Section Boundaries

- Treat front matter as a dedicated metadata section.
- Treat pre-heading content as a preamble section.
- Each heading starts a section that runs until the next heading of the same or higher level.
- Preserve heading hierarchy, page ranges, tables, code blocks, and lists.
- A file without headings is one section.

## Provenance

Every material claim should cite the original source path and a concrete location:

- Markdown: `[source: <relative-path> § <heading-path-or-preamble>, lines <start>-<end>]`
- Paginated formats: use page ranges instead of lines.

## Output Set

Write the smallest complete set that proves the corpus is covered:

1. Typology files for `ADR`, `PRD`, and `SPEC_RFC`
2. Landmark files for `ADR`, `PRD`, and `SPEC_RFC`
3. `taxonomy-index.md`
4. `manifest.json`
5. `section-classifications.jsonl`
6. `coverage-report.md`
7. `review-queue.md`
8. `source-map.json`

## Archive Gates

- Do not archive originals until inventory, classification, verification, synthesis, and output audit all pass.
- Recompute hashes before archive promotion.
- Preserve excluded policy files and generated outputs.
- Fail closed on missing, mismatched, or unverified items.
