---
name: document-taxonomy-corpus
description: Consolidate a documentation corpus into evidence-backed ADR, PRD, and SPEC_RFC taxonomy outputs with section-level classification, provenance, verification, and archive gating. Use when turning docs folders, planning corpora, or mixed engineering documentation into inventories, rubrics, classifications, typologies, landmarks, manifests, coverage reports, review queues, and archive plans.
---

# Document Taxonomy Corpus

Use this skill to turn a documentation corpus into a traceable taxonomy package without flattening contradictions or inventing missing evidence.

## Workflow

1. Inventory the corpus.
   - Enumerate documents, assets, unsupported files, and section counts.
   - Record source paths, hashes, and archive eligibility.
   - Block if the docs root is not ready or the scope is ambiguous.

2. Build the rubric.
   - Classify section evidence into `ADR`, `PRD`, `SPEC_RFC`, `MIXED`, or `NON_TAXONOMIC`.
   - Keep the taxonomy strict: technical blueprints belong in `SPEC_RFC`, product intent in `PRD`, durable decisions in `ADR`.
   - Use the reference file for landmarks, section boundaries, and provenance rules.

3. Classify sections.
   - Work section by section, not just document by document.
   - Capture heading path, line or page range, content hash, semantic role, rationale, detected landmarks, and review flags.
   - Mark ambiguous or low-confidence sections for human review instead of forcing a category.

4. Verify results.
   - Check coverage, provenance, taxonomy fit, and confidence thresholds.
   - Preserve conflicts and negative evidence explicitly.
   - Repair failed sections before synthesis.

5. Synthesize outputs.
   - Produce typology and landmark files for ADRs, PRDs, and SPECs/RFCs.
   - Produce the control files: index, manifest, section JSONL, coverage report, review queue, and source map.
   - Keep all claims cited back to original source sections.

6. Archive only after gates pass.
   - Never archive originals until inventory, classification, synthesis, and output audit all pass.
   - Verify hashes before any move or deletion.

## Operating Rules

- Treat `NON_TAXONOMIC` as the safe off-ramp for navigation, changelogs, glossaries, licenses, and similar material.
- Use `MIXED` only when one indivisible section truly contains inseparable evidence for multiple categories.
- Do not merge contradictions into a single clean story.
- Keep original paths, hashes, and citations intact in every artifact.
- Prefer the smallest complete output set that proves coverage and provenance.

## References

- [Workflow contract](references/workflow-contract.md): category landmarks, section boundaries, provenance format, output set, and archive gates.
