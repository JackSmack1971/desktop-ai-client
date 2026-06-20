export const meta = {
  name: "document_taxonomy_corpus",
  description: "Classifies every document section under docs, synthesizes ADR/PRD/SPEC-RFC typologies and landmarks, verifies coverage, then archives originals safely.",
  whenToUse: "Run when consolidating a documentation corpus into evidence-backed ADR, PRD, and SPEC/RFC taxonomies while preserving the original source files.",
  phases: [
    { title: "Preflight & Inventory", detail: "Inventory every source item, hash files, validate scope, and estimate work." },
    { title: "Rubric Construction", detail: "Build and independently verify the section-level classification rubric." },
    { title: "Section Classification", detail: "Fan out one context-aware classifier per document and persist structured evidence." },
    { title: "Adversarial Verification", detail: "Check section coverage, taxonomy accuracy, provenance, and confidence; repair failures." },
    { title: "Corpus Synthesis", detail: "Create individual ADR, PRD, and SPEC/RFC typology and landmark files." },
    { title: "Completeness Gate", detail: "Generate manifests and audit every document, section, claim, and required output." },
    { title: "Archive Originals", detail: "Stage, hash-verify, and archive the original corpus only after all gates pass." }
  ]
};

const EMBEDDED_TAXONOMY = `
CLASSIFICATION PURPOSE
Classify engineering documentation at section level, then infer a document-level dominant typology without forcing ambiguous or generic material into a false class.

ADR — ARCHITECTURAL DECISION RECORD
Intent: a focused, usually single-topic, historically durable record of an architecturally significant choice and its trade-offs.
Strong landmarks: Context and Problem Statement; Considered Options; Decision Outcome; Consequences; Confirmation.
Metadata landmarks: status, date, decision-makers, consulted, informed.
Naming cues: docs/decisions/NNNN-title.md, adr-NNNN.md, NNNN-imperative-phrase.md.
Audience/mutability: current and future engineering teams; normally immutable and superseded by a new record.
Positive semantic evidence: an explicit decision, rejected alternatives, rationale, trade-offs, consequences, confirmation criteria.
Negative evidence: a broad implementation blueprint without a single isolated decision is normally not an ADR.

PRD — PRODUCT REQUIREMENTS DOCUMENT
Intent: define product purpose, user value, desired behavior, scope, and success before or during delivery.
Strong landmarks: Objectives or OKRs; KPIs or Success Metrics; Target Personas; In Scope / Out of Scope; User Stories; Functional Requirements; Non-Functional Requirements; Dependencies; Timeline; Open Questions.
Metadata landmarks: target_release_date, status, epic_link, product_manager, core team.
Naming cues: prd-feature.md, feature-requirements.md, requirements.md, docs/product/prd-*.md.
Audience/mutability: product, engineering, design, and business stakeholders; living document.
Positive semantic evidence: user problem, outcomes, measurable success, personas, acceptance behavior, business or product constraints.
Negative evidence: detailed APIs, schemas, component interactions, deployment topology, or reference implementation generally indicate SPEC/RFC even when requirements context is present.

SPEC_RFC — TECHNICAL SPECIFICATION / REQUEST FOR COMMENTS
Intent: provide a detailed engineering blueprint for implementing a feature, system, protocol, or architectural shift.
Strong landmarks: Summary; Motivation or Background; System Architecture; Components and Interactions; API Contracts; Database Schema; Data Model; Sequence or State Flows; Testing Strategy; Rollout or Deployment; Drawbacks; Alternatives; Reference Implementation.
Metadata landmarks: authors, created_date, status, reviewers.
Naming cues: rfc-NNNN.md, spec-topic.md, tech-spec-project.md.
Audience/mutability: implementers and peer technical reviewers; semi-immutable during implementation.
Positive semantic evidence: concrete technical design, interfaces, algorithms, data structures, migration steps, failure modes, testing, observability, rollout.
Tie-breaker: technical indicators such as database schemas, API contracts, component sequence flows, deployment blueprints, or implementation mechanics dominate PRD-style context and require SPEC_RFC unless the section only states product intent.

SAFE OFFRAMPS
MIXED: use only when one indivisible section contains substantial, inseparable evidence for multiple typologies and splitting it further would destroy meaning.
NON_TAXONOMIC: use for navigation, changelogs, glossaries, general introductions, references, licenses, meeting notes, or other material that is confidently outside ADR/PRD/SPEC_RFC.
Do not use low confidence as a substitute for careful analysis. Low-confidence sections must be routed to review.

SECTION BOUNDARIES
Treat YAML/TOML front matter as a dedicated metadata section. Treat content before the first heading as a preamble section. For Markdown-like files, each heading begins a section that includes content until the next heading of the same or higher level; preserve heading hierarchy. Tables, code blocks, diagrams, and lists belong to the active section. A file without headings is one section. For paginated formats, preserve page ranges. Never omit empty-but-semantic headings.

EVIDENCE AND PROVENANCE
Every classification must cite concrete source landmarks and preserve source path, heading path, line or page range, and a content hash. Rationale must distinguish positive evidence, negative evidence, and tie-breakers. Never invent missing content. Contradictions must remain explicit rather than being silently merged.
`;

const CATEGORY_ENUM = ["ADR", "PRD", "SPEC_RFC", "MIXED", "NON_TAXONOMIC"];

const INVENTORY_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    status: { type: "string", enum: ["READY", "BLOCKED"] },
    docsRoot: { type: "string" },
    inventoryArtifactPath: { type: "string" },
    gitRepository: { type: "boolean" },
    docsTreeClean: { type: "boolean" },
    guidelinePathFound: { type: "boolean" },
    totalEntries: { type: "integer", minimum: 0 },
    documentCount: { type: "integer", minimum: 0 },
    assetCount: { type: "integer", minimum: 0 },
    unsupportedDocumentCount: { type: "integer", minimum: 0 },
    estimatedSectionCount: { type: "integer", minimum: 0 },
    blockingIssues: { type: "array", items: { type: "string" } },
    warnings: { type: "array", items: { type: "string" } },
    entries: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          id: { type: "string" },
          sourcePath: { type: "string" },
          relativePath: { type: "string" },
          kind: { type: "string", enum: ["DOCUMENT", "ASSET", "SYMLINK"] },
          format: { type: "string" },
          byteSize: { type: "integer", minimum: 0 },
          sha256: { type: "string" },
          supported: { type: "boolean" },
          archiveEligible: { type: "boolean" },
          classificationArtifactPath: { type: "string" },
          notes: { type: "array", items: { type: "string" } }
        },
        required: ["id", "sourcePath", "relativePath", "kind", "format", "byteSize", "sha256", "supported", "archiveEligible", "classificationArtifactPath", "notes"]
      }
    }
  },
  required: ["status", "docsRoot", "inventoryArtifactPath", "gitRepository", "docsTreeClean", "guidelinePathFound", "totalEntries", "documentCount", "assetCount", "unsupportedDocumentCount", "estimatedSectionCount", "blockingIssues", "warnings", "entries"]
};

const RUBRIC_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    status: { type: "string", enum: ["VERIFIED", "REJECTED"] },
    rubricArtifactPath: { type: "string" },
    categories: { type: "array", items: { type: "string", enum: CATEGORY_ENUM } },
    confidenceThreshold: { type: "number", minimum: 0, maximum: 1 },
    requiredProvenanceFields: { type: "array", items: { type: "string" } },
    tieBreakers: { type: "array", items: { type: "string" } },
    segmentationRules: { type: "array", items: { type: "string" } },
    verifierFindings: { type: "array", items: { type: "string" } }
  },
  required: ["status", "rubricArtifactPath", "categories", "confidenceThreshold", "requiredProvenanceFields", "tieBreakers", "segmentationRules", "verifierFindings"]
};

const LOCATION_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    startLine: { type: ["integer", "null"], minimum: 1 },
    endLine: { type: ["integer", "null"], minimum: 1 },
    startPage: { type: ["integer", "null"], minimum: 1 },
    endPage: { type: ["integer", "null"], minimum: 1 }
  },
  required: ["startLine", "endLine", "startPage", "endPage"]
};

const CLASSIFICATION_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    artifactVersion: { type: "string" },
    artifactPath: { type: "string" },
    documentId: { type: "string" },
    source: {
      type: "object",
      additionalProperties: false,
      properties: {
        path: { type: "string" },
        relativePath: { type: "string" },
        sha256: { type: "string" },
        format: { type: "string" },
        title: { type: "string" },
        byteSize: { type: "integer", minimum: 0 },
        sectioningMethod: { type: "string" }
      },
      required: ["path", "relativePath", "sha256", "format", "title", "byteSize", "sectioningMethod"]
    },
    documentClassification: {
      type: "object",
      additionalProperties: false,
      properties: {
        primaryCategory: { type: "string", enum: CATEGORY_ENUM },
        secondaryCategories: { type: "array", uniqueItems: true, items: { type: "string", enum: CATEGORY_ENUM } },
        confidence: { type: "number", minimum: 0, maximum: 1 },
        rationale: { type: "string" },
        dominantLandmarks: { type: "array", items: { type: "string" } },
        missingExpectedElements: { type: "array", items: { type: "string" } },
        ambiguityNotes: { type: "array", items: { type: "string" } }
      },
      required: ["primaryCategory", "secondaryCategories", "confidence", "rationale", "dominantLandmarks", "missingExpectedElements", "ambiguityNotes"]
    },
    sections: {
      type: "array",
      minItems: 1,
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          sectionId: { type: "string" },
          ordinal: { type: "integer", minimum: 1 },
          heading: { type: "string" },
          headingPath: { type: "array", items: { type: "string" } },
          headingLevel: { type: ["integer", "null"], minimum: 1, maximum: 6 },
          location: LOCATION_SCHEMA,
          contentSha256: { type: "string" },
          primaryCategory: { type: "string", enum: CATEGORY_ENUM },
          secondaryCategories: { type: "array", uniqueItems: true, items: { type: "string", enum: CATEGORY_ENUM } },
          categoryScores: {
            type: "object",
            additionalProperties: false,
            properties: {
              ADR: { type: "number", minimum: 0, maximum: 1 },
              PRD: { type: "number", minimum: 0, maximum: 1 },
              SPEC_RFC: { type: "number", minimum: 0, maximum: 1 }
            },
            required: ["ADR", "PRD", "SPEC_RFC"]
          },
          confidence: { type: "number", minimum: 0, maximum: 1 },
          semanticRole: { type: "string" },
          detectedLandmarks: {
            type: "array",
            items: {
              type: "object",
              additionalProperties: false,
              properties: {
                category: { type: "string", enum: ["ADR", "PRD", "SPEC_RFC"] },
                landmark: { type: "string" },
                evidence: { type: "string" }
              },
              required: ["category", "landmark", "evidence"]
            }
          },
          rationale: { type: "string" },
          negativeEvidence: { type: "array", items: { type: "string" } },
          missingExpectedElements: { type: "array", items: { type: "string" } },
          crossReferences: { type: "array", items: { type: "string" } },
          needsHumanReview: { type: "boolean" }
        },
        required: ["sectionId", "ordinal", "heading", "headingPath", "headingLevel", "location", "contentSha256", "primaryCategory", "secondaryCategories", "categoryScores", "confidence", "semanticRole", "detectedLandmarks", "rationale", "negativeEvidence", "missingExpectedElements", "crossReferences", "needsHumanReview"]
      }
    },
    coverage: {
      type: "object",
      additionalProperties: false,
      properties: {
        discoveredSectionCount: { type: "integer", minimum: 1 },
        classifiedSectionCount: { type: "integer", minimum: 1 },
        unclassifiedSectionCount: { type: "integer", minimum: 0 },
        gapCount: { type: "integer", minimum: 0 },
        overlapCount: { type: "integer", minimum: 0 },
        coveragePercent: { type: "number", minimum: 0, maximum: 100 }
      },
      required: ["discoveredSectionCount", "classifiedSectionCount", "unclassifiedSectionCount", "gapCount", "overlapCount", "coveragePercent"]
    },
    warnings: { type: "array", items: { type: "string" } }
  },
  required: ["artifactVersion", "artifactPath", "documentId", "source", "documentClassification", "sections", "coverage", "warnings"]
};

const VERIFICATION_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    documentId: { type: "string" },
    sourcePath: { type: "string" },
    classificationArtifactPath: { type: "string" },
    verificationArtifactPath: { type: "string" },
    passed: { type: "boolean" },
    sectionCoveragePassed: { type: "boolean" },
    provenancePassed: { type: "boolean" },
    taxonomyPassed: { type: "boolean" },
    confidencePolicyPassed: { type: "boolean" },
    sourceHashMatched: { type: "boolean" },
    discoveredSectionCount: { type: "integer", minimum: 0 },
    classifiedSectionCount: { type: "integer", minimum: 0 },
    reviewSectionIds: { type: "array", items: { type: "string" } },
    blockingIssues: { type: "array", items: { type: "string" } },
    nonBlockingNotes: { type: "array", items: { type: "string" } },
    repairInstructions: { type: "array", items: { type: "string" } }
  },
  required: ["documentId", "sourcePath", "classificationArtifactPath", "verificationArtifactPath", "passed", "sectionCoveragePassed", "provenancePassed", "taxonomyPassed", "confidencePolicyPassed", "sourceHashMatched", "discoveredSectionCount", "classifiedSectionCount", "reviewSectionIds", "blockingIssues", "nonBlockingNotes", "repairInstructions"]
};

const SYNTHESIS_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    category: { type: "string", enum: ["ADR", "PRD", "SPEC_RFC"] },
    typologyPath: { type: "string" },
    landmarkPath: { type: "string" },
    written: { type: "boolean" },
    sourceDocumentCount: { type: "integer", minimum: 0 },
    sourceSectionCount: { type: "integer", minimum: 0 },
    conflictCount: { type: "integer", minimum: 0 },
    reviewItemCount: { type: "integer", minimum: 0 },
    omittedSections: { type: "array", items: { type: "string" } },
    notes: { type: "array", items: { type: "string" } }
  },
  required: ["category", "typologyPath", "landmarkPath", "written", "sourceDocumentCount", "sourceSectionCount", "conflictCount", "reviewItemCount", "omittedSections", "notes"]
};

const FINALIZATION_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    written: { type: "boolean" },
    indexPath: { type: "string" },
    manifestPath: { type: "string" },
    sectionJsonlPath: { type: "string" },
    coverageReportPath: { type: "string" },
    reviewQueuePath: { type: "string" },
    sourceMapPath: { type: "string" },
    documentCount: { type: "integer", minimum: 0 },
    sectionCount: { type: "integer", minimum: 0 },
    reviewCount: { type: "integer", minimum: 0 },
    assetCount: { type: "integer", minimum: 0 }
  },
  required: ["written", "indexPath", "manifestPath", "sectionJsonlPath", "coverageReportPath", "reviewQueuePath", "sourceMapPath", "documentCount", "sectionCount", "reviewCount", "assetCount"]
};

const AUDIT_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    passed: { type: "boolean" },
    auditArtifactPath: { type: "string" },
    sixRequiredFilesPresent: { type: "boolean" },
    sourceHashIntegrityPassed: { type: "boolean" },
    documentCoveragePercent: { type: "number", minimum: 0, maximum: 100 },
    sectionCoveragePercent: { type: "number", minimum: 0, maximum: 100 },
    provenanceCoveragePercent: { type: "number", minimum: 0, maximum: 100 },
    unresolvedReviewCount: { type: "integer", minimum: 0 },
    unsupportedDocumentCount: { type: "integer", minimum: 0 },
    blockingIssues: { type: "array", items: { type: "string" } },
    repairInstructions: { type: "array", items: { type: "string" } },
    warnings: { type: "array", items: { type: "string" } }
  },
  required: ["passed", "auditArtifactPath", "sixRequiredFilesPresent", "sourceHashIntegrityPassed", "documentCoveragePercent", "sectionCoveragePercent", "provenanceCoveragePercent", "unresolvedReviewCount", "unsupportedDocumentCount", "blockingIssues", "repairInstructions", "warnings"]
};

const REPAIR_SUMMARY_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    completed: { type: "boolean" },
    filesModified: { type: "array", items: { type: "string" } },
    issuesAddressed: { type: "array", items: { type: "string" } },
    remainingRisks: { type: "array", items: { type: "string" } }
  },
  required: ["completed", "filesModified", "issuesAddressed", "remainingRisks"]
};

const ARCHIVE_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    status: { type: "string", enum: ["COMPLETED", "FAILED"] },
    mode: { type: "string", enum: ["move", "copy"] },
    archiveRoot: { type: "string" },
    originalsRoot: { type: "string" },
    archiveManifestPath: { type: "string" },
    expectedItemCount: { type: "integer", minimum: 0 },
    stagedItemCount: { type: "integer", minimum: 0 },
    verifiedItemCount: { type: "integer", minimum: 0 },
    removedSourceCount: { type: "integer", minimum: 0 },
    hashMismatchCount: { type: "integer", minimum: 0 },
    missingItemCount: { type: "integer", minimum: 0 },
    errors: { type: "array", items: { type: "string" } }
  },
  required: ["status", "mode", "archiveRoot", "originalsRoot", "archiveManifestPath", "expectedItemCount", "stagedItemCount", "verifiedItemCount", "removedSourceCount", "hashMismatchCount", "missingItemCount", "errors"]
};

const POST_ARCHIVE_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    passed: { type: "boolean" },
    outputFilesIntact: { type: "boolean" },
    archiveHashesIntact: { type: "boolean" },
    sourceStateCorrect: { type: "boolean" },
    checkedItemCount: { type: "integer", minimum: 0 },
    issues: { type: "array", items: { type: "string" } },
    restorationRequired: { type: "boolean" }
  },
  required: ["passed", "outputFilesIntact", "archiveHashesIntact", "sourceStateCorrect", "checkedItemCount", "issues", "restorationRequired"]
};

function runtimeArgs() {
  return (typeof args !== "undefined" && args && typeof args === "object") ? args : {};
}

function clampInteger(value, fallback, min, max) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(parsed)));
}

function clampNumber(value, fallback, min, max) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, Math.min(max, parsed));
}

function normalizeResult(value) {
  if (value === null || value === undefined) return value;
  if (typeof value !== "string") return value;
  try {
    return JSON.parse(value);
  } catch (_) {
    return value;
  }
}

function modelOptions(base, modelName) {
  if (modelName && String(modelName).trim() !== "") {
    return { ...base, model: String(modelName).trim() };
  }
  return base;
}

function tokenTotal() {
  if (typeof budget === "undefined" || !budget || typeof budget !== "object") return 0;
  if (typeof budget.total === "number") return budget.total;
  if (typeof budget.tokens === "number") return budget.tokens;
  if (budget.usage && typeof budget.usage.total === "number") return budget.usage.total;
  return 0;
}

function assertBudget(config, stageName) {
  const used = tokenTotal();
  if (config.maxTokenBudget > 0 && used > config.maxTokenBudget) {
    throw new Error(`Token budget exceeded during ${stageName}: ${used} > ${config.maxTokenBudget}`);
  }
}

async function parallelInBatches(items, batchSize, config, stageName, workerFactory) {
  const outcomes = [];
  for (let offset = 0; offset < items.length; offset += batchSize) {
    assertBudget(config, stageName);
    const batch = items.slice(offset, offset + batchSize);
    log(`${stageName}: processing ${offset + 1}-${offset + batch.length} of ${items.length}`);
    const batchResults = await parallel(batch.map((item, localIndex) => async () => {
      try {
        const result = await workerFactory(item, offset + localIndex);
        return { ok: true, itemId: item.id || String(offset + localIndex + 1), result: normalizeResult(result) };
      } catch (error) {
        return { ok: false, itemId: item.id || String(offset + localIndex + 1), error: error && error.message ? error.message : String(error) };
      }
    }));
    outcomes.push(...batchResults.filter(Boolean));
  }
  return outcomes;
}

function buildConfig() {
  const input = runtimeArgs();
  const docsRoot = input.docsRoot || "./docs";
  const outputRoot = input.outputRoot || `${docsRoot}/_taxonomy`;
  const runId = input.runId || "taxonomy-migration";
  const stagingRoot = input.stagingRoot || `./.claude/runs/document-taxonomy/${runId}`;
  const archiveRoot = input.archiveRoot || `${docsRoot}/_archive/${runId}`;
  const archiveMode = input.archiveMode === "copy" ? "copy" : "move";
  return {
    docsRoot,
    outputRoot,
    runId,
    stagingRoot,
    archiveRoot,
    archiveMode,
    guidelinesPath: input.guidelinesPath || "",
    classifierModel: input.classifierModel || "",
    verifierModel: input.verifierModel || "",
    synthesisModel: input.synthesisModel || "",
    dryRun: input.dryRun === true,
    resume: input.resume === true,
    force: input.force === true,
    requireGit: input.requireGit !== false,
    requireCleanDocs: input.requireCleanDocs !== false,
    archiveOriginals: input.archiveOriginals !== false,
    blockArchiveOnReview: input.blockArchiveOnReview !== false,
    confidenceThreshold: clampNumber(input.confidenceThreshold, 0.80, 0.50, 0.99),
    batchSize: clampInteger(input.batchSize, 8, 1, 16),
    maxDocuments: clampInteger(input.maxDocuments, 500, 1, 1000),
    maxRepairRounds: clampInteger(input.maxRepairRounds, 2, 0, 4),
    maxOutputRepairRounds: clampInteger(input.maxOutputRepairRounds, 2, 0, 4),
    maxTokenBudget: clampInteger(input.maxTokenBudget, 8000000, 0, 100000000),
    includeExtensions: Array.isArray(input.includeExtensions)
      ? input.includeExtensions
      : [".md", ".markdown", ".mdx", ".txt", ".rst", ".adoc", ".html", ".htm", ".pdf", ".docx"],
    additionalExcludes: Array.isArray(input.excludePaths) ? input.excludePaths : []
  };
}

async function inventoryCorpus(config) {
  phase("Preflight & Inventory");
  const prompt = `
You are the deterministic preflight and inventory agent for a documentation taxonomy migration.

SCOPE
- Recursively inventory every regular file and symlink under: ${config.docsRoot}
- Never follow symlinks outside the tree.
- Exclude generated/output roots: ${config.outputRoot}, ${config.archiveRoot}, ${config.stagingRoot}
- Exclude these additional paths or path prefixes: ${JSON.stringify(config.additionalExcludes)}
- If the optional guideline path is inside docs, treat it as a policy input, not a corpus document: ${config.guidelinesPath || "<none>"}
- Document extensions: ${JSON.stringify(config.includeExtensions)}
- All other files are ASSET records. Assets are not section-classified but remain archive-eligible so the original corpus is preserved.

REQUIRED CHECKS
1. Confirm docsRoot exists and is readable.
2. Detect whether this is a Git repository and whether the docs tree is clean. Include modified, deleted, renamed, and untracked status.
3. Refuse generated-root collisions unless resume=${config.resume} or force=${config.force}.
4. Assign stable, deterministic IDs in lexicographic relative-path order: DOC-0001, ASSET-0001, LINK-0001.
5. Compute SHA-256 for every regular file. For symlinks, hash the link target text without following it.
6. Estimate section count from headings/pages, but do not classify content.
7. For each DOCUMENT, determine whether current tools can extract its text losslessly enough for section analysis. Mark unsupported when extraction is impossible.
8. Set classificationArtifactPath for documents to ${config.stagingRoot}/classifications/<ID>.json. Assets and symlinks use an empty string.
9. Set archiveEligible=false only for explicit exclusions and generated files; all original corpus entries should be true.
10. In live mode, create ${config.stagingRoot} and write the complete returned object exactly to ${config.stagingRoot}/inventory.json. In dry-run mode, do not write.

BLOCKING POLICY
- requireGit=${config.requireGit}
- requireCleanDocs=${config.requireCleanDocs}
- maxDocuments=${config.maxDocuments}
- The status is BLOCKED if Git is required but absent, clean docs are required but dirty, docsRoot is missing, documentCount exceeds the cap, unsupportedDocumentCount > 0, or output/archive roots collide unsafely.
- Do not edit, move, copy, or delete any corpus source file.
- Return only schema-compliant structured data.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: "inventory docs", schema: INVENTORY_SCHEMA }, config.classifierModel)));
}

async function constructRubric(config) {
  phase("Rubric Construction");
  const prompt = `
Create the canonical section-classification rubric for this run and independently self-audit it before returning.

BASELINE FRAMEWORK
${EMBEDDED_TAXONOMY}

OPTIONAL PROJECT GUIDELINE
Path: ${config.guidelinesPath || "<not supplied>"}
If a non-empty path exists, read it in full and use it as the controlling source. Reconcile it with the embedded baseline without weakening its ADR/PRD/SPEC-RFC landmarks, structured-output requirements, confidence offramps, or SPEC-over-PRD technical tie-breaker. If the path is absent, use the embedded baseline.

OUTPUT REQUIREMENTS
- Write a complete machine-readable rubric to ${config.stagingRoot}/rubric.json.
- Include category definitions, positive and negative landmarks, metadata and naming cues, section segmentation rules, provenance requirements, confidence policy, ambiguity policy, and tie-breakers.
- confidenceThreshold=${config.confidenceThreshold}
- Categories must be exactly ADR, PRD, SPEC_RFC, MIXED, NON_TAXONOMIC.
- A second independent review pass must compare the rubric against the controlling framework and record verifierFindings.
- Status is REJECTED if any major landmark, tie-breaker, safe offramp, or provenance rule is missing.
- Do not touch corpus source files.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: "build rubric", schema: RUBRIC_SCHEMA }, config.verifierModel || config.classifierModel)));
}

async function classifyDocument(entry, config, rubricPath) {
  const prompt = `
You are a context-preserving document analyst. Analyze exactly one source document and classify every section.

SOURCE
- documentId: ${entry.id}
- path: ${entry.sourcePath}
- relative path: ${entry.relativePath}
- expected SHA-256: ${entry.sha256}
- format: ${entry.format}
- expected byte size: ${entry.byteSize}
- output artifact: ${entry.classificationArtifactPath}

CONTROLLING RUBRIC
Read in full: ${rubricPath}
Also apply this non-negotiable tie-breaker: detailed APIs, schemas, component interactions, algorithms, deployment topology, migration mechanics, or implementation/testing design dominate PRD-like context and classify as SPEC_RFC.

METHOD
1. Recompute the source hash before analysis. Do not proceed on mismatch.
2. Read/extract the entire document. Do not classify from only the beginning and end.
3. Segment deterministically:
   - front matter is its own section;
   - pre-heading prose is a preamble section;
   - preserve heading hierarchy and exact line ranges for text formats;
   - preserve page ranges for paginated formats;
   - tables, code blocks, lists, and diagrams belong to their active section;
   - a heading with no body is still a section;
   - a headingless document is one section.
4. Classify each section using the complete document as context. Score ADR, PRD, and SPEC_RFC independently from 0 to 1.
5. Use MIXED only for genuinely inseparable multi-typology sections. Use NON_TAXONOMIC for confidently generic or out-of-taxonomy material.
6. Set needsHumanReview=true when confidence < ${config.confidenceThreshold}, source extraction is lossy, evidence conflicts, or boundaries are uncertain.
7. Preserve positive evidence, negative evidence, missing expected elements, cross-references, source coordinates, and a SHA-256 of each exact section text.
8. Infer document-level primary and secondary typologies from the verified section evidence; do not let filenames override content.
9. Coverage must be 100%, with zero gaps, zero overlaps, and no unclassified sections.
10. Write the returned JSON exactly to ${entry.classificationArtifactPath}, then reread it and confirm it parses and matches the returned object.

ACCURACY RULES
- Never summarize away disagreements.
- Never invent rationale, requirements, decisions, APIs, or implementation details.
- Keep source excerpts in evidence brief but specific.
- Do not edit the source document.
- Return only schema-compliant structured data.
`;
  return await agent(prompt, modelOptions({ label: `classify ${entry.id}`, schema: CLASSIFICATION_SCHEMA }, config.classifierModel));
}

async function verifyDocument(entry, config, rubricPath) {
  const verificationPath = `${config.stagingRoot}/verifications/${entry.id}.json`;
  const prompt = `
Act as an independent adversarial verifier. You did not create the classification.

READ ALL THREE INPUTS
1. Source document: ${entry.sourcePath}
2. Canonical rubric: ${rubricPath}
3. Classification artifact: ${entry.classificationArtifactPath}

VERIFY
- Recompute source SHA-256 and require ${entry.sha256}.
- Independently segment the full source and compare every front-matter, preamble, heading, line/page range, table, code block, and terminal section.
- Require exact complete coverage with no missing, duplicate, overlapping, or invented sections.
- Verify every section hash against the exact source slice.
- Re-evaluate category scores, primary category, landmarks, negative evidence, and rationale.
- Enforce SPEC_RFC dominance when concrete technical design appears, even alongside user stories or scope.
- Reject filename-only classification.
- Confirm MIXED and NON_TAXONOMIC are used narrowly and correctly.
- Confirm document-level classification is consistent with section evidence.
- confidenceThreshold=${config.confidenceThreshold}; blockArchiveOnReview=${config.blockArchiveOnReview}.
- If blockArchiveOnReview is true, any needsHumanReview section makes confidencePolicyPassed=false and passed=false.
- Record precise, executable repair instructions.
- Write the returned verification exactly to ${verificationPath}.
- Do not edit source or classification artifacts.
`;
  return await agent(prompt, modelOptions({ label: `verify ${entry.id}`, schema: VERIFICATION_SCHEMA }, config.verifierModel || config.synthesisModel));
}

async function repairDocument(entry, verification, config, rubricPath, round) {
  const prompt = `
Repair one section-classification artifact after an independent audit.

INPUTS
- Source: ${entry.sourcePath}
- Expected source SHA-256: ${entry.sha256}
- Rubric: ${rubricPath}
- Existing artifact: ${entry.classificationArtifactPath}
- Verification artifact: ${verification.verificationArtifactPath}
- Repair round: ${round}

REQUIREMENTS
- Read the source, rubric, existing classification, and verification report in full.
- Correct every blocking issue and follow every repair instruction.
- Re-segment from the source when coverage, line/page ranges, or hashes are questioned.
- Do not merely change labels to satisfy the checker; classification must be evidence-based.
- Maintain exact source provenance and 100% section coverage.
- Recompute document and section hashes.
- Overwrite ${entry.classificationArtifactPath} atomically with the corrected returned JSON, reread it, and validate it.
- Never edit the source document.
`;
  return await agent(prompt, modelOptions({ label: `repair ${entry.id}`, schema: CLASSIFICATION_SCHEMA }, config.classifierModel || config.synthesisModel));
}

function categoryFileNames(category, outputRoot) {
  if (category === "ADR") {
    return { typology: `${outputRoot}/typologies/ADRs.md`, landmarks: `${outputRoot}/landmarks/ADR-Taxonomic-Landmarks.md` };
  }
  if (category === "PRD") {
    return { typology: `${outputRoot}/typologies/PRDs.md`, landmarks: `${outputRoot}/landmarks/PRD-Taxonomic-Landmarks.md` };
  }
  return { typology: `${outputRoot}/typologies/SPECs-RFCs.md`, landmarks: `${outputRoot}/landmarks/SPEC-RFC-Taxonomic-Landmarks.md` };
}

async function synthesizeCategory(category, config, rubricPath) {
  const paths = categoryFileNames(category, config.outputRoot);
  const prompt = `
Synthesize the verified corpus evidence for category ${category} into TWO separate Markdown files.

INPUTS
- Inventory: ${config.stagingRoot}/inventory.json
- Rubric: ${rubricPath}
- Verified classification artifacts: ${config.stagingRoot}/classifications/*.json
- Verification artifacts: ${config.stagingRoot}/verifications/*.json
- Original source documents remain available under ${config.docsRoot} during this phase.

OUTPUT 1 — DOCUMENT TYPOLOGY
Write: ${paths.typology}
Create an evidence-backed corpus synthesis for ${category}. Include:
- purpose and observed scope;
- canonical corpus narrative organized by coherent themes;
- all decisions/requirements/specifications attributed to source sections;
- dependencies, constraints, unresolved questions, and cross-document relationships;
- explicit conflict matrix when sources disagree;
- missing expected material and confidence caveats;
- source coverage appendix.
Do not flatten contradictions or convert proposals into accepted facts.

OUTPUT 2 — TAXONOMIC LANDMARKS
Write: ${paths.landmarks}
Create the empirical landmark profile for ${category}. Include:
- observed filenames and directory cues;
- metadata fields;
- heading and structural patterns;
- semantic and lexical indicators;
- audience and mutability signals;
- positive evidence, negative evidence, and confusion boundaries;
- expected landmarks missing from the corpus;
- drift and false-positive risks;
- source-by-source landmark table.
Clearly distinguish canonical landmarks from corpus-observed frequency.

PROVENANCE FORMAT
Every material synthesized claim must cite one or more original sections using:
[source: <original-relative-path> § <heading-path-or-preamble>, lines <start>-<end>]
or page ranges for paginated formats. Use original paths, not staging artifact paths. Claims supported by multiple sources cite each source. No uncited factual synthesis.

FILTERING
- Include sections whose primaryCategory is ${category}.
- Include MIXED sections only where ${category} is a secondary category and the relevant statements are separable and cited.
- Exclude NON_TAXONOMIC content except where needed to explain navigation or provenance.
- Do not use any classification artifact that failed verification.
- If there are zero qualifying sections, still write both files with an explicit evidence-empty statement and the expected canonical landmarks from the rubric, clearly labeled as canonical rather than observed.

Write atomically, reread both files, and return the summary.
`;
  return await agent(prompt, modelOptions({ label: `synthesize ${category}`, schema: SYNTHESIS_SCHEMA }, config.synthesisModel || config.verifierModel));
}

async function finalizeOutputs(config, rubricPath, synthesisResults) {
  const prompt = `
Generate the corpus-wide control files after the three category syntheses.

READ
- ${config.stagingRoot}/inventory.json
- ${rubricPath}
- ${config.stagingRoot}/classifications/*.json
- ${config.stagingRoot}/verifications/*.json
- Category synthesis summaries: ${JSON.stringify(synthesisResults)}
- All six files under ${config.outputRoot}/typologies and ${config.outputRoot}/landmarks

WRITE ATOMICALLY
1. ${config.outputRoot}/taxonomy-index.md
   Cross-link all six files; summarize corpus counts, dominant typologies, cross-category dependencies, conflicts, exclusions, and review status.
2. ${config.outputRoot}/manifest.json
   Include runId, configuration relevant to interpretation, input hashes, artifact paths, output paths, counts, category distributions, review flags, and intended archive destinations. Do not include secrets.
3. ${config.outputRoot}/section-classifications.jsonl
   One valid JSON object per source section with document ID, source path, heading path, coordinates, hashes, categories, scores, confidence, landmarks, and review flag.
4. ${config.outputRoot}/coverage-report.md
   Prove document and section coverage, list assets, unsupported items, failed or repaired items, confidence distribution, and exact definition-of-done status.
5. ${config.outputRoot}/review-queue.md
   List every unresolved low-confidence, lossy-extraction, MIXED, conflict, or verifier-warning item with source coordinates and recommended human action. Write an explicit empty state when none exist.
6. ${config.outputRoot}/source-map.json
   Map each original path and SHA-256 to its classification artifact, synthesized category files, and intended archive path under ${config.archiveRoot}/originals/<relative-path>.

INTEGRITY
- Use only verified artifacts.
- Preserve original relative paths and hashes.
- JSONL must parse line by line.
- Every document must appear in manifest, coverage report, and source map.
- Every classified section must appear exactly once in JSONL.
- Do not edit or archive originals.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: "finalize corpus", schema: FINALIZATION_SCHEMA }, config.synthesisModel || config.verifierModel)));
}

async function auditOutputs(config, rubricPath, round) {
  const auditPath = `${config.stagingRoot}/output-audit-round-${round}.json`;
  const prompt = `
Perform a strict independent completeness and accuracy audit of the documentation taxonomy migration.

INPUTS
- Inventory: ${config.stagingRoot}/inventory.json
- Rubric: ${rubricPath}
- Classifications: ${config.stagingRoot}/classifications/*.json
- Verifications: ${config.stagingRoot}/verifications/*.json
- Output root: ${config.outputRoot}
- Required files:
  ${config.outputRoot}/typologies/ADRs.md
  ${config.outputRoot}/typologies/PRDs.md
  ${config.outputRoot}/typologies/SPECs-RFCs.md
  ${config.outputRoot}/landmarks/ADR-Taxonomic-Landmarks.md
  ${config.outputRoot}/landmarks/PRD-Taxonomic-Landmarks.md
  ${config.outputRoot}/landmarks/SPEC-RFC-Taxonomic-Landmarks.md
  ${config.outputRoot}/taxonomy-index.md
  ${config.outputRoot}/manifest.json
  ${config.outputRoot}/section-classifications.jsonl
  ${config.outputRoot}/coverage-report.md
  ${config.outputRoot}/review-queue.md
  ${config.outputRoot}/source-map.json

AUDIT CRITERIA
1. Recompute source hashes and compare to inventory while originals are still in place.
2. Require every DOCUMENT to have a passing verification and every discovered section to appear exactly once in JSONL.
3. Require 100% document coverage and 100% section coverage.
4. Validate every JSON and JSONL file.
5. Validate all six requested category files exist and are non-empty.
6. Sample and trace every material claim type in each synthesis back to a real source coordinate and hash. Reject fabricated, uncited, or misattributed claims.
7. Verify category inclusion/exclusion rules, SPEC_RFC tie-breakers, conflict preservation, and canonical-vs-observed landmark labeling.
8. Require unsupportedDocumentCount=0.
9. If blockArchiveOnReview=${config.blockArchiveOnReview}, require unresolvedReviewCount=0.
10. Write the returned audit exactly to ${auditPath}.

A pass requires zero blocking issues. Do not edit files in this audit step.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: `audit outputs ${round}`, schema: AUDIT_SCHEMA }, config.verifierModel || config.synthesisModel)));
}

async function repairOutputs(config, rubricPath, auditResult, round) {
  const prompt = `
Repair the generated taxonomy outputs after a failed independent audit.

READ
- Inventory: ${config.stagingRoot}/inventory.json
- Rubric: ${rubricPath}
- Verified classifications and verifications under ${config.stagingRoot}
- All generated files under ${config.outputRoot}
- Audit report: ${auditResult.auditArtifactPath}
- Repair instructions: ${JSON.stringify(auditResult.repairInstructions)}
- Blocking issues: ${JSON.stringify(auditResult.blockingIssues)}
- Repair round: ${round}

Fix every issue at its source. Regenerate affected category files, JSON/JSONL manifests, citations, coverage calculations, or source mappings as necessary. Do not weaken the rubric, remove difficult sections, suppress conflicts, or mark unresolved work as complete. Do not edit or archive original files. Write atomically and validate all modified files before returning.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: `repair outputs ${round}`, schema: REPAIR_SUMMARY_SCHEMA }, config.synthesisModel || config.verifierModel)));
}

async function archiveOriginals(config) {
  phase("Archive Originals");
  const prompt = `
Archive the original documentation corpus using a two-phase, hash-verified procedure.

PRECONDITIONS
- Read inventory: ${config.stagingRoot}/inventory.json
- Read passing output audit: ${config.stagingRoot}/output-audit-round-*.json and use the latest passing one.
- Read source map: ${config.outputRoot}/source-map.json
- Archive mode: ${config.archiveMode}
- Archive root: ${config.archiveRoot}
- Final originals root: ${config.archiveRoot}/originals
- Resume: ${config.resume}; Force: ${config.force}
- Never archive ${config.outputRoot}, ${config.archiveRoot}, ${config.stagingRoot}, or the optional policy file ${config.guidelinesPath || "<none>"}.
- Operate only on inventory entries where archiveEligible=true.

TRANSACTIONAL PROCEDURE
1. Abort before mutation if no passing output audit exists, source hashes differ from inventory, or the final archive root already exists unexpectedly.
2. Create ${config.archiveRoot}/.staging and copy every archive-eligible item there while preserving relative paths, bytes, permissions where possible, and symlink identity without following links.
3. Recompute every staged hash/link-target hash and compare with inventory. Require exact expected/staged/verified counts and zero mismatches.
4. Atomically promote .staging to ${config.archiveRoot}/originals only after every item verifies.
5. Write ${config.archiveRoot}/archive-manifest.json with original path, archive path, kind, byte size, original hash, archive hash, and status for every item.
6. If mode=copy, leave sources in place.
7. If mode=move, only after the complete promoted archive and manifest verify, delete original inventory entries one by one. Never delete generated outputs or excluded policy files. Remove empty original directories only when they contain no excluded/generated content.
8. On any failure before promotion, remove only .staging and leave every source untouched.
9. On deletion failure after promotion, stop; retain the complete archive and report exactly which sources remain. Never delete an unverified item.
10. Return only structured status.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: "archive originals", schema: ARCHIVE_SCHEMA }, config.verifierModel || config.synthesisModel)));
}

async function verifyArchive(config, archiveResult) {
  const prompt = `
Independently verify the completed archive and generated outputs.

READ
- Inventory: ${config.stagingRoot}/inventory.json
- Output manifest: ${config.outputRoot}/manifest.json
- Archive manifest: ${archiveResult.archiveManifestPath}
- Archive root: ${archiveResult.originalsRoot}
- Archive mode: ${config.archiveMode}

CHECK
- Every archive-eligible inventory item exists at the exact relative archive path with matching hash or link target.
- All required generated taxonomy files remain present and unchanged.
- In copy mode, original sources still exist and match hashes.
- In move mode, original inventory file paths are absent, except explicitly excluded/generated paths.
- Counts match and there are zero missing or extra manifest records.
- Do not modify anything.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: "verify archive", schema: POST_ARCHIVE_SCHEMA }, config.verifierModel || config.synthesisModel)));
}

async function restoreFromArchive(config, archiveResult, postArchiveResult) {
  const prompt = `
CRITICAL RECOVERY: the post-archive verification failed.

Archive: ${archiveResult.originalsRoot}
Archive manifest: ${archiveResult.archiveManifestPath}
Issues: ${JSON.stringify(postArchiveResult.issues)}

For every missing or hash-mismatched original source path, restore the verified archived item to its exact original relative path. Never overwrite a differing existing file; report it instead. Do not remove the archive. Do not modify generated outputs. Validate restored hashes and return a repair summary.
`;
  return normalizeResult(await agent(prompt, modelOptions({ label: "restore originals", schema: REPAIR_SUMMARY_SCHEMA }, config.verifierModel || config.synthesisModel)));
}

async function runWorkflow() {
  const config = buildConfig();
  log(`Document taxonomy run '${config.runId}' targeting ${config.docsRoot}`);

  const inventory = await inventoryCorpus(config);
  if (!inventory || inventory.status !== "READY") {
    return { status: "BLOCKED", phase: "inventory", config, details: inventory };
  }
  if (inventory.documentCount === 0) {
    return { status: "BLOCKED", phase: "inventory", reason: "No documents found", config, details: inventory };
  }
  if (config.dryRun) {
    return {
      status: "DRY_RUN_COMPLETE",
      config,
      inventorySummary: {
        totalEntries: inventory.totalEntries,
        documentCount: inventory.documentCount,
        assetCount: inventory.assetCount,
        estimatedSectionCount: inventory.estimatedSectionCount,
        warnings: inventory.warnings
      },
      nextAction: "Run again with dryRun=false after reviewing scope and budget."
    };
  }

  assertBudget(config, "rubric construction");
  const rubric = await constructRubric(config);
  if (!rubric || rubric.status !== "VERIFIED") {
    return { status: "BLOCKED", phase: "rubric", config, details: rubric };
  }

  const documents = inventory.entries.filter((entry) => entry.kind === "DOCUMENT" && entry.supported);
  phase("Section Classification");
  const classificationRuns = await parallelInBatches(
    documents,
    config.batchSize,
    config,
    "Section classification",
    (entry) => classifyDocument(entry, config, rubric.rubricArtifactPath)
  );
  const classificationFailures = classificationRuns.filter((item) => !item.ok);
  if (classificationFailures.length > 0) {
    return { status: "BLOCKED", phase: "classification", failures: classificationFailures, config };
  }

  phase("Adversarial Verification");
  let verificationRuns = await parallelInBatches(
    documents,
    config.batchSize,
    config,
    "Adversarial verification",
    (entry) => verifyDocument(entry, config, rubric.rubricArtifactPath)
  );

  let verificationById = {};
  for (const run of verificationRuns) {
    if (run.ok && run.result && run.result.documentId) verificationById[run.result.documentId] = run.result;
  }

  for (let round = 1; round <= config.maxRepairRounds; round++) {
    const failedDocuments = documents.filter((entry) => {
      const verification = verificationById[entry.id];
      return !verification || verification.passed !== true;
    });
    if (failedDocuments.length === 0) break;

    log(`Document repair round ${round}: ${failedDocuments.length} document(s)`);
    const repairRuns = await parallelInBatches(
      failedDocuments,
      config.batchSize,
      config,
      `Document repair ${round}`,
      (entry) => repairDocument(entry, verificationById[entry.id] || { verificationArtifactPath: "", blockingIssues: ["Verifier did not return a result"] }, config, rubric.rubricArtifactPath, round)
    );
    const repairFailures = repairRuns.filter((item) => !item.ok);
    if (repairFailures.length > 0) {
      log(`Repair round ${round} had ${repairFailures.length} agent failure(s).`);
    }

    const reverifyRuns = await parallelInBatches(
      failedDocuments,
      config.batchSize,
      config,
      `Reverification ${round}`,
      (entry) => verifyDocument(entry, config, rubric.rubricArtifactPath)
    );
    for (const run of reverifyRuns) {
      if (run.ok && run.result && run.result.documentId) verificationById[run.result.documentId] = run.result;
    }
  }

  const unresolvedDocuments = documents.filter((entry) => !verificationById[entry.id] || verificationById[entry.id].passed !== true);
  if (unresolvedDocuments.length > 0) {
    return {
      status: "BLOCKED",
      phase: "verification",
      unresolved: unresolvedDocuments.map((entry) => ({ entry, verification: verificationById[entry.id] || null })),
      config
    };
  }

  phase("Corpus Synthesis");
  assertBudget(config, "corpus synthesis");
  const categoryResultsRaw = await parallel(["ADR", "PRD", "SPEC_RFC"].map((category) => async () => {
    try {
      return normalizeResult(await synthesizeCategory(category, config, rubric.rubricArtifactPath));
    } catch (error) {
      return { category, written: false, error: error && error.message ? error.message : String(error) };
    }
  }));
  const categoryResults = categoryResultsRaw.filter(Boolean);
  if (categoryResults.length !== 3 || categoryResults.some((result) => result.written !== true)) {
    return { status: "BLOCKED", phase: "synthesis", categoryResults, config };
  }

  const finalization = await finalizeOutputs(config, rubric.rubricArtifactPath, categoryResults);
  if (!finalization || finalization.written !== true) {
    return { status: "BLOCKED", phase: "finalization", finalization, config };
  }

  phase("Completeness Gate");
  let outputAudit = await auditOutputs(config, rubric.rubricArtifactPath, 0);
  for (let round = 1; outputAudit && outputAudit.passed !== true && round <= config.maxOutputRepairRounds; round++) {
    await repairOutputs(config, rubric.rubricArtifactPath, outputAudit, round);
    outputAudit = await auditOutputs(config, rubric.rubricArtifactPath, round);
  }
  if (!outputAudit || outputAudit.passed !== true) {
    return { status: "BLOCKED", phase: "output-audit", outputAudit, config };
  }

  let archiveResult = null;
  let postArchive = null;
  if (config.archiveOriginals) {
    archiveResult = await archiveOriginals(config);
    if (!archiveResult || archiveResult.status !== "COMPLETED" || archiveResult.hashMismatchCount !== 0 || archiveResult.missingItemCount !== 0) {
      return { status: "BLOCKED", phase: "archive", archiveResult, config };
    }
    postArchive = await verifyArchive(config, archiveResult);
    if (!postArchive || postArchive.passed !== true) {
      const restoration = await restoreFromArchive(config, archiveResult, postArchive || { issues: ["Archive verifier did not return a result"] });
      return { status: "RECOVERED_AFTER_ARCHIVE_FAILURE", phase: "post-archive", archiveResult, postArchive, restoration, config };
    }
  }

  return {
    status: "COMPLETED",
    runId: config.runId,
    docsRoot: config.docsRoot,
    outputRoot: config.outputRoot,
    archiveRoot: config.archiveOriginals ? config.archiveRoot : null,
    archiveMode: config.archiveOriginals ? config.archiveMode : null,
    counts: {
      sourceEntries: inventory.totalEntries,
      documents: inventory.documentCount,
      assets: inventory.assetCount,
      sections: finalization.sectionCount,
      reviewItems: finalization.reviewCount
    },
    outputs: {
      typologies: categoryResults.map((result) => result.typologyPath),
      landmarks: categoryResults.map((result) => result.landmarkPath),
      index: finalization.indexPath,
      manifest: finalization.manifestPath,
      sectionClassifications: finalization.sectionJsonlPath,
      coverageReport: finalization.coverageReportPath,
      reviewQueue: finalization.reviewQueuePath,
      sourceMap: finalization.sourceMapPath,
      outputAudit: outputAudit.auditArtifactPath,
      archiveManifest: archiveResult ? archiveResult.archiveManifestPath : null
    },
    integrity: {
      documentCoveragePercent: outputAudit.documentCoveragePercent,
      sectionCoveragePercent: outputAudit.sectionCoveragePercent,
      provenanceCoveragePercent: outputAudit.provenanceCoveragePercent,
      archiveVerified: postArchive ? postArchive.passed : null
    }
  };
}

return await runWorkflow();
