export const meta = {
  name: "audit-claude-context-compression",
  description: "Audits project-level .claude configuration for context-loading correctness, token efficiency, semantic fidelity, safety, and compression opportunities without modifying source files.",
  whenToUse: "Run when .claude instructions, rules, skills, agents, workflows, hooks, or settings have grown large, conflicting, unsafe, or expensive to load.",
  phases: [
    {
      title: "Scope and Inventory",
      detail: "Enforces the .claude-only boundary and inventories every eligible file."
    },
    {
      title: "Context Surface Mapping",
      detail: "Models startup, conditional, deferred, and excluded context surfaces."
    },
    {
      title: "Parallel Fidelity Audit",
      detail: "Audits deterministic file batches for correctness, redundancy, safety, and compression potential."
    },
    {
      title: "Adversarial Verification",
      detail: "Uses independent reviewers and a deterministic vote gate to suppress weak findings."
    },
    {
      title: "Synthesis and Artifacts",
      detail: "Writes a cited report, machine-readable findings, coverage evidence, and a safe compression plan."
    }
  ]
};

const AUDIT_SCHEMA_VERSION = "1.0.0";
const FIXED_TARGET_ROOT = ".claude";
const DEFAULT_OUTPUT_DIR = ".claude/audits/context-compression";
const RUNTIME_CONCURRENCY_LIMIT = 16;
const DEFAULT_WORKER_LIMIT = 10;
const DEFAULT_BATCH_SIZE = 8;
const DEFAULT_MAX_FILES = 300;
const DEFAULT_MAX_TOTAL_BYTES = 3_000_000;
const DEFAULT_MAX_ESTIMATED_TOKENS = 1_500_000;
const DEFAULT_MAX_FINDINGS = 180;
const DEFAULT_VERIFIER_COUNT = 3;

const SEVERITY_ORDER = {
  critical: 5,
  high: 4,
  medium: 3,
  low: 2,
  info: 1
};

const INVENTORY_SCHEMA = {
  type: "object",
  properties: {
    targetRoot: { type: "string" },
    focusPaths: { type: "array", items: { type: "string" } },
    discoveredFileCount: { type: "integer", minimum: 0 },
    eligibleFileCount: { type: "integer", minimum: 0 },
    totalBytes: { type: "integer", minimum: 0 },
    truncated: { type: "boolean" },
    files: {
      type: "array",
      items: {
        type: "object",
        properties: {
          path: { type: "string" },
          kind: {
            type: "string",
            enum: [
              "project-memory",
              "unscoped-rule",
              "path-scoped-rule",
              "skill-entry",
              "skill-support",
              "command",
              "agent-definition",
              "agent-memory",
              "workflow",
              "settings",
              "hook",
              "output-style",
              "other"
            ]
          },
          bytes: { type: "integer", minimum: 0 },
          lines: { type: "integer", minimum: 0 },
          startupLoad: { type: "boolean" },
          conditionalLoad: { type: "boolean" },
          deferredLoad: { type: "boolean" },
          hasFrontmatter: { type: "boolean" },
          pathsFrontmatter: { type: "boolean" },
          disableModelInvocation: { type: "boolean" },
          imports: {
            type: "array",
            items: {
              type: "object",
              properties: {
                raw: { type: "string" },
                resolvedPath: { type: "string" },
                inScope: { type: "boolean" },
                depthHint: { type: "integer", minimum: 0 }
              },
              required: ["raw", "resolvedPath", "inScope", "depthHint"]
            }
          },
          parseWarnings: { type: "array", items: { type: "string" } }
        },
        required: [
          "path",
          "kind",
          "bytes",
          "lines",
          "startupLoad",
          "conditionalLoad",
          "deferredLoad",
          "hasFrontmatter",
          "pathsFrontmatter",
          "disableModelInvocation",
          "imports",
          "parseWarnings"
        ]
      }
    },
    excluded: {
      type: "array",
      items: {
        type: "object",
        properties: {
          path: { type: "string" },
          reason: { type: "string" }
        },
        required: ["path", "reason"]
      }
    },
    externalBoundaries: {
      type: "array",
      items: {
        type: "object",
        properties: {
          layer: { type: "string" },
          expectedLocation: { type: "string" },
          auditStatus: { type: "string", enum: ["excluded-by-scope", "referenced-only"] },
          reason: { type: "string" }
        },
        required: ["layer", "expectedLocation", "auditStatus", "reason"]
      }
    },
    warnings: { type: "array", items: { type: "string" } }
  },
  required: [
    "targetRoot",
    "focusPaths",
    "discoveredFileCount",
    "eligibleFileCount",
    "totalBytes",
    "truncated",
    "files",
    "excluded",
    "externalBoundaries",
    "warnings"
  ]
};

const CONTEXT_MAP_SCHEMA = {
  type: "object",
  properties: {
    startupFiles: { type: "array", items: { type: "string" } },
    conditionalFiles: { type: "array", items: { type: "string" } },
    deferredFiles: { type: "array", items: { type: "string" } },
    nonContextFiles: { type: "array", items: { type: "string" } },
    externalReferences: {
      type: "array",
      items: {
        type: "object",
        properties: {
          sourcePath: { type: "string" },
          rawImport: { type: "string" },
          resolvedPath: { type: "string" },
          risk: { type: "string", enum: ["low", "medium", "high"] },
          reason: { type: "string" }
        },
        required: ["sourcePath", "rawImport", "resolvedPath", "risk", "reason"]
      }
    },
    metrics: {
      type: "object",
      properties: {
        startupBytes: { type: "integer", minimum: 0 },
        conditionalBytes: { type: "integer", minimum: 0 },
        deferredBytes: { type: "integer", minimum: 0 },
        estimatedStartupTokens: { type: "integer", minimum: 0 },
        estimatedDeferredMetadataTokens: { type: "integer", minimum: 0 }
      },
      required: [
        "startupBytes",
        "conditionalBytes",
        "deferredBytes",
        "estimatedStartupTokens",
        "estimatedDeferredMetadataTokens"
      ]
    },
    loadSemanticsNotes: { type: "array", items: { type: "string" } },
    coverageWarnings: { type: "array", items: { type: "string" } }
  },
  required: [
    "startupFiles",
    "conditionalFiles",
    "deferredFiles",
    "nonContextFiles",
    "externalReferences",
    "metrics",
    "loadSemanticsNotes",
    "coverageWarnings"
  ]
};

const BATCH_AUDIT_SCHEMA = {
  type: "object",
  properties: {
    batchId: { type: "string" },
    filesReviewed: { type: "array", items: { type: "string" } },
    coverage: {
      type: "array",
      items: {
        type: "object",
        properties: {
          path: { type: "string" },
          status: { type: "string", enum: ["reviewed", "missing", "unreadable", "excluded"] },
          note: { type: "string" }
        },
        required: ["path", "status", "note"]
      }
    },
    findings: {
      type: "array",
      items: {
        type: "object",
        properties: {
          localId: { type: "string" },
          filePath: { type: "string" },
          lineStart: { type: "integer", minimum: 0 },
          lineEnd: { type: "integer", minimum: 0 },
          severity: { type: "string", enum: ["critical", "high", "medium", "low", "info"] },
          category: {
            type: "string",
            enum: [
              "scope-boundary",
              "load-semantics",
              "contradiction",
              "redundancy",
              "stale-guidance",
              "ambiguity",
              "token-cost",
              "import-integrity",
              "frontmatter",
              "json-validity",
              "security",
              "permissions",
              "hook-safety",
              "skill-discovery",
              "agent-definition",
              "workflow-reliability",
              "compression-candidate",
              "lossy-compression-risk",
              "documentation",
              "coverage"
            ]
          },
          title: { type: "string" },
          description: { type: "string" },
          evidence: { type: "string" },
          impact: { type: "string" },
          recommendation: { type: "string" },
          technique: {
            type: "string",
            enum: [
              "none",
              "code-block-masking",
              "python-ast-interface",
              "telegraph-hieratic-sidecar",
              "pakt-table-sidecar",
              "deduplicate",
              "path-scope-rule",
              "defer-to-skill",
              "disable-model-invocation",
              "progressive-disclosure",
              "split-supporting-files",
              "settings-or-hook-enforcement"
            ]
          },
          estimatedBeforeTokens: { type: "integer", minimum: 0 },
          estimatedAfterTokens: { type: "integer", minimum: 0 },
          fidelityRisk: { type: "string", enum: ["none", "low", "medium", "high"] },
          autoApplicable: { type: "boolean" },
          confidence: { type: "number", minimum: 0, maximum: 1 },
          provenance: { type: "string", enum: ["direct-file", "cross-file", "inferred"] }
        },
        required: [
          "localId",
          "filePath",
          "lineStart",
          "lineEnd",
          "severity",
          "category",
          "title",
          "description",
          "evidence",
          "impact",
          "recommendation",
          "technique",
          "estimatedBeforeTokens",
          "estimatedAfterTokens",
          "fidelityRisk",
          "autoApplicable",
          "confidence",
          "provenance"
        ]
      }
    },
    batchSummary: { type: "string" },
    warnings: { type: "array", items: { type: "string" } }
  },
  required: ["batchId", "filesReviewed", "coverage", "findings", "batchSummary", "warnings"]
};

const CANONICAL_FINDINGS_SCHEMA = {
  type: "object",
  properties: {
    findings: {
      type: "array",
      items: {
        type: "object",
        properties: {
          findingId: { type: "string" },
          filePath: { type: "string" },
          lineStart: { type: "integer", minimum: 0 },
          lineEnd: { type: "integer", minimum: 0 },
          severity: { type: "string", enum: ["critical", "high", "medium", "low", "info"] },
          category: { type: "string" },
          title: { type: "string" },
          description: { type: "string" },
          evidence: { type: "string" },
          impact: { type: "string" },
          recommendation: { type: "string" },
          technique: { type: "string" },
          estimatedBeforeTokens: { type: "integer", minimum: 0 },
          estimatedAfterTokens: { type: "integer", minimum: 0 },
          fidelityRisk: { type: "string", enum: ["none", "low", "medium", "high"] },
          autoApplicable: { type: "boolean" },
          confidence: { type: "number", minimum: 0, maximum: 1 },
          sourceBatchIds: { type: "array", items: { type: "string" } }
        },
        required: [
          "findingId",
          "filePath",
          "lineStart",
          "lineEnd",
          "severity",
          "category",
          "title",
          "description",
          "evidence",
          "impact",
          "recommendation",
          "technique",
          "estimatedBeforeTokens",
          "estimatedAfterTokens",
          "fidelityRisk",
          "autoApplicable",
          "confidence",
          "sourceBatchIds"
        ]
      }
    },
    duplicateGroups: {
      type: "array",
      items: {
        type: "object",
        properties: {
          keptFindingId: { type: "string" },
          mergedLocalIds: { type: "array", items: { type: "string" } },
          rationale: { type: "string" }
        },
        required: ["keptFindingId", "mergedLocalIds", "rationale"]
      }
    },
    coverage: {
      type: "object",
      properties: {
        expectedFiles: { type: "array", items: { type: "string" } },
        reviewedFiles: { type: "array", items: { type: "string" } },
        missingFiles: { type: "array", items: { type: "string" } },
        unreadableFiles: { type: "array", items: { type: "string" } }
      },
      required: ["expectedFiles", "reviewedFiles", "missingFiles", "unreadableFiles"]
    },
    warnings: { type: "array", items: { type: "string" } }
  },
  required: ["findings", "duplicateGroups", "coverage", "warnings"]
};

const REVIEW_SCHEMA = {
  type: "object",
  properties: {
    reviewer: { type: "string" },
    decisions: {
      type: "array",
      items: {
        type: "object",
        properties: {
          findingId: { type: "string" },
          verdict: { type: "string", enum: ["confirm", "weaken", "reject", "needs-manual"] },
          correctedSeverity: { type: "string", enum: ["critical", "high", "medium", "low", "info"] },
          confidence: { type: "number", minimum: 0, maximum: 1 },
          rationale: { type: "string" }
        },
        required: ["findingId", "verdict", "correctedSeverity", "confidence", "rationale"]
      }
    },
    coverageConcerns: { type: "array", items: { type: "string" } },
    reviewerSummary: { type: "string" }
  },
  required: ["reviewer", "decisions", "coverageConcerns", "reviewerSummary"]
};

const WRITER_SCHEMA = {
  type: "object",
  properties: {
    status: { type: "string", enum: ["completed", "completed-with-warnings", "failed-to-write"] },
    writtenFiles: { type: "array", items: { type: "string" } },
    headline: { type: "string" },
    score: { type: "integer", minimum: 0, maximum: 100 },
    verifiedFindingCount: { type: "integer", minimum: 0 },
    suppressedFindingCount: { type: "integer", minimum: 0 },
    criticalCount: { type: "integer", minimum: 0 },
    highCount: { type: "integer", minimum: 0 },
    estimatedStartupTokens: { type: "integer", minimum: 0 },
    estimatedSavingsTokens: { type: "integer", minimum: 0 },
    warnings: { type: "array", items: { type: "string" } }
  },
  required: [
    "status",
    "writtenFiles",
    "headline",
    "score",
    "verifiedFindingCount",
    "suppressedFindingCount",
    "criticalCount",
    "highCount",
    "estimatedStartupTokens",
    "estimatedSavingsTokens",
    "warnings"
  ]
};

function clampInteger(value, fallback, minimum, maximum) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  const integer = Math.floor(parsed);
  return Math.max(minimum, Math.min(maximum, integer));
}

function normalizeRelativePath(value) {
  if (typeof value !== "string") return null;
  const raw = value.trim().replace(/\\/g, "/");
  if (!raw || raw.startsWith("/") || /^[A-Za-z]:\//.test(raw) || raw.startsWith("~")) return null;
  const parts = [];
  for (const segment of raw.replace(/^\.\//, "").split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") return null;
    parts.push(segment);
  }
  return parts.join("/");
}

function isInsideClaude(path) {
  return path === FIXED_TARGET_ROOT || path.startsWith(`${FIXED_TARGET_ROOT}/`);
}

function safeOutputDir(value) {
  const normalized = normalizeRelativePath(value || DEFAULT_OUTPUT_DIR);
  if (!normalized || !isInsideClaude(normalized)) return null;
  return normalized;
}

function safeFocusPaths(value) {
  if (value === undefined || value === null) return [];
  const incoming = Array.isArray(value) ? value : [value];
  const normalized = [];
  for (const item of incoming) {
    const path = normalizeRelativePath(String(item));
    if (!path || !isInsideClaude(path)) return null;
    if (!normalized.includes(path)) normalized.push(path);
  }
  return normalized;
}

function chunk(items, size) {
  const groups = [];
  for (let index = 0; index < items.length; index += size) {
    groups.push(items.slice(index, index + size));
  }
  return groups;
}

function serialize(value) {
  return JSON.stringify(value, null, 2);
}

function currentBudgetTokens() {
  if (typeof budget === "undefined" || budget === null) return null;
  if (typeof budget.total === "number") return budget.total;
  if (typeof budget.tokens === "number") return budget.tokens;
  if (typeof budget.used === "number") return budget.used;
  return null;
}

function tokenEstimateFromBytes(bytes) {
  return Math.max(0, Math.ceil(bytes / 3.7));
}

function projectedWorkflowTokens(inventory, auditAgentCount, verifierCount) {
  const sourceTokens = tokenEstimateFromBytes(inventory.totalBytes || 0);
  const workerOverhead = auditAgentCount * 3_500;
  const verificationOverhead = verifierCount * Math.min(120_000, Math.max(12_000, sourceTokens * 0.45));
  const synthesisOverhead = Math.min(160_000, Math.max(18_000, sourceTokens * 0.6));
  return Math.ceil(sourceTokens * 1.35 + workerOverhead + verificationOverhead + synthesisOverhead);
}

function severityRank(value) {
  return SEVERITY_ORDER[value] || 0;
}

function medianSeverity(values, fallback) {
  const valid = values.filter((value) => severityRank(value) > 0);
  if (valid.length === 0) return fallback;
  const sorted = valid.slice().sort((a, b) => severityRank(a) - severityRank(b));
  return sorted[Math.floor(sorted.length / 2)];
}

function aggregateVerification(canonicalFindings, reviews) {
  const decisionMaps = reviews.map((review) => {
    const map = {};
    for (const decision of review.decisions || []) map[decision.findingId] = decision;
    return map;
  });

  const verified = [];
  const suppressed = [];

  for (const finding of canonicalFindings) {
    const decisions = decisionMaps
      .map((map) => map[finding.findingId])
      .filter(Boolean);

    const counts = { confirm: 0, weaken: 0, reject: 0, "needs-manual": 0 };
    const correctedSeverities = [];
    const rationales = [];

    for (const decision of decisions) {
      counts[decision.verdict] = (counts[decision.verdict] || 0) + 1;
      correctedSeverities.push(decision.correctedSeverity);
      rationales.push({
        verdict: decision.verdict,
        confidence: decision.confidence,
        rationale: decision.rationale
      });
    }

    const record = {
      ...finding,
      originalSeverity: finding.severity,
      severity: medianSeverity(correctedSeverities, finding.severity),
      verification: {
        reviewerCount: decisions.length,
        votes: counts,
        rationales
      }
    };

    if (counts.reject >= 2) {
      suppressed.push({ ...record, suppressionReason: "Rejected by at least two independent reviewers." });
      continue;
    }

    if (counts.confirm >= 2) record.verificationStatus = "confirmed";
    else if (counts.weaken >= 2) record.verificationStatus = "weakened";
    else record.verificationStatus = "needs-manual-review";

    verified.push(record);
  }

  verified.sort((a, b) => {
    const severityDelta = severityRank(b.severity) - severityRank(a.severity);
    if (severityDelta !== 0) return severityDelta;
    return b.confidence - a.confidence;
  });

  return { verified, suppressed };
}

function countBySeverity(findings, severity) {
  return findings.filter((finding) => finding.severity === severity).length;
}

function totalEstimatedSavings(findings) {
  return findings.reduce((sum, finding) => {
    const before = Number(finding.estimatedBeforeTokens) || 0;
    const after = Number(finding.estimatedAfterTokens) || 0;
    return sum + Math.max(0, before - after);
  }, 0);
}

function auditPrompt(batchId, files, contextMap, findingLimit) {
  return `You are a read-only Claude Code configuration auditor.

SCOPE CONTRACT (NON-NEGOTIABLE)
- Read only regular files whose lexical paths are listed in TARGET FILES and are under .claude/.
- Never read ~/.claude, parent directories, repository-root CLAUDE.md, CLAUDE.local.md, .mcp.json, .worktreeinclude, source code, or an import target outside .claude/.
- Do not follow symlinks that resolve outside .claude/.
- Do not write, edit, rename, delete, execute project scripts, install packages, invoke network tools, or change git state.
- An external @import may be reported but must not be opened.
- Every file listed must receive a coverage record, even when missing or unreadable.

AUDIT OBJECTIVE
Audit every target for context-loading correctness, semantic conflicts, context cost, deterministic compression opportunities, and production safety. Findings require direct evidence with file path and line range. Do not invent line numbers; use 0 only when a precise line is unavailable.

AUTHORITATIVE LOAD MODEL FOR THIS AUDIT
1. .claude/CLAUDE.md is project instruction context.
2. .claude/rules/**/*.md without YAML 'paths' load unconditionally; rules with 'paths' are conditional.
3. Skill name/description metadata may be discoverable before invocation; full SKILL.md content is deferred until invocation. 'disable-model-invocation: true' removes automatic model invocation/discovery context.
4. Supporting skill files are progressive-disclosure assets and should not be treated as startup-loaded unless imported by an always-loaded file.
5. @imports are active only outside inline/fenced code; resolve relative to the containing file; recursive depth must not exceed four hops. Do not open out-of-scope targets.
6. Auto-memory, user rules, global skills, ancestry CLAUDE.md files, project-root CLAUDE.local.md, and root .mcp.json are outside this run's read boundary.
7. Instructions are behavioral context, not hard enforcement. Security invariants belong in settings permissions or deterministic hooks.

COMPRESSION RUBRIC
- Code-block masking: recommend when prose transformations surround fenced code; code content must restore byte-for-byte.
- Python AST interface compression: only for derived context views of Python assets; retain imports, signatures, annotations, decorators, and docstrings; never replace source-of-truth files.
- Telegraph/Hieratic rewriting: only as a reversible, validated sidecar or cache. Reject suggestions that overwrite nuanced source instructions without equivalence tests.
- PAKT-style tabular serialization: only for regular tables with exact row/column recovery and escaping rules.
- Prefer lower-risk reductions first: deduplication, path-scoped rules, move procedures to skills, supporting-file progressive disclosure, concise descriptions, and disable-model-invocation for manual side-effect skills.
- Token counts are estimates. Never present them as tokenizer-exact.

SAFETY RUBRIC
Check malformed YAML/JSON, broad permissions, secret-path access, unsafe hooks, workflows without bounds/schemas/verification, side-effect skills auto-invocable by the model, contradictory settings, stale commands, and instructions that claim enforcement without hooks/settings.

CROSS-FILE CONTEXT MAP
${serialize(contextMap)}

BATCH ID
${batchId}

TARGET FILES
${serialize(files)}

OUTPUT DISCIPLINE
- Return at most ${findingLimit} high-value findings for this batch.
- Prefer one canonical finding over repeated symptoms.
- Evidence excerpts must be short and must not expose secrets.
- 'autoApplicable' may be true only for low-risk, deterministic, reversible changes.
- Use provenance 'inferred' sparingly and with lower confidence.
- Return only schema-compliant structured output.`;
}

async function runAudit() {
  const rawArgs = typeof args === "undefined" || args === null ? {} : args;
  const requestedTarget = normalizeRelativePath(rawArgs.targetRoot || FIXED_TARGET_ROOT);
  const outputDir = safeOutputDir(rawArgs.outputDir);
  const focusPaths = safeFocusPaths(rawArgs.focusPaths);

  if (requestedTarget !== FIXED_TARGET_ROOT) {
    return {
      status: "rejected",
      reason: "targetRoot must be exactly .claude; this workflow intentionally refuses broader repository or user-home scope."
    };
  }
  if (!outputDir) {
    return {
      status: "rejected",
      reason: "outputDir must be a relative path contained within .claude/."
    };
  }
  if (focusPaths === null) {
    return {
      status: "rejected",
      reason: "Every focus path must be a relative path contained within .claude/."
    };
  }

  const config = {
    targetRoot: FIXED_TARGET_ROOT,
    outputDir,
    focusPaths,
    maxFiles: clampInteger(rawArgs.maxFiles, DEFAULT_MAX_FILES, 1, 900),
    maxTotalBytes: clampInteger(rawArgs.maxTotalBytes, DEFAULT_MAX_TOTAL_BYTES, 10_000, 20_000_000),
    maxEstimatedTokens: clampInteger(
      rawArgs.maxEstimatedTokens,
      DEFAULT_MAX_ESTIMATED_TOKENS,
      50_000,
      8_000_000
    ),
    maxFindings: clampInteger(rawArgs.maxFindings, DEFAULT_MAX_FINDINGS, 10, 500),
    batchSize: clampInteger(rawArgs.batchSize, DEFAULT_BATCH_SIZE, 1, 20),
    workerLimit: clampInteger(
      rawArgs.workerLimit,
      DEFAULT_WORKER_LIMIT,
      1,
      RUNTIME_CONCURRENCY_LIMIT - 2
    ),
    verifierCount: clampInteger(rawArgs.verifierCount, DEFAULT_VERIFIER_COUNT, 3, 3),
    writeArtifacts: rawArgs.writeArtifacts !== false,
    model: typeof rawArgs.model === "string" && rawArgs.model.trim() ? rawArgs.model.trim() : undefined,
    reviewerModel:
      typeof rawArgs.reviewerModel === "string" && rawArgs.reviewerModel.trim()
        ? rawArgs.reviewerModel.trim()
        : undefined
  };

  phase("Scope and Inventory");
  log("Enforcing the project-level .claude-only audit boundary.");

  const inventory = await agent(
    `Inventory the project-level .claude directory for a context-compression audit.

STRICT SCOPE
- Read only .claude/** and only the focus paths listed below when non-empty.
- Never read repository-root files, parent directories, ~/.claude/**, .mcp.json, .worktreeinclude, source code, or external @import targets.
- Do not follow a symlink whose resolved target leaves .claude/.
- Do not modify any file or run any state-changing command.
- Exclude ${outputDir}/**, .claude/runs/**, .claude/worktrees/**, .claude/cache/**, .claude/tmp/**, binary files, and generated dependency folders.

FOCUS PATHS
${serialize(config.focusPaths)}

LIMITS
- Enumerate no more than ${config.maxFiles} eligible files in the returned files array.
- Set truncated=true and report discoveredFileCount when the actual eligible count exceeds that limit.
- Capture byte and line counts without returning full file contents.

CLASSIFICATION
Classify .claude/CLAUDE.md, unscoped/path-scoped rules, SKILL.md entries and supporting files, commands, agent definitions, agent-memory, workflows, settings JSON, hook scripts, output styles, and other files.
For Markdown, inspect YAML frontmatter and active @imports outside code spans/fences. Resolve import paths lexically relative to the containing file but do not open an import target unless it remains under .claude/ and is itself an eligible inventory file.
For settings, inventory only the file and parse warnings; do not follow references outside scope.

EXTERNAL BOUNDARIES TO RECORD WITHOUT READING
- ~/.claude/CLAUDE.md
- ~/.claude/rules/**/*.md
- ancestry CLAUDE.md and CLAUDE.local.md files outside .claude/
- ~/.claude/projects/<project>/memory/MEMORY.md and topic files
- ~/.claude/skills/**
- project-root .mcp.json and .worktreeinclude
- managed settings and CLI/environment overrides

Return only schema-compliant structured output.`,
    {
      label: "inventory claude files",
      phase: "Scope and Inventory",
      schema: INVENTORY_SCHEMA,
      ...(config.model ? { model: config.model } : {})
    }
  );

  if (!inventory || !Array.isArray(inventory.files)) {
    return { status: "aborted", reason: "Inventory agent returned no usable file list." };
  }
  if (inventory.truncated || inventory.discoveredFileCount > config.maxFiles) {
    return {
      status: "rejected-scope-too-large",
      reason: `Discovered ${inventory.discoveredFileCount} files, exceeding maxFiles=${config.maxFiles}. Use focusPaths or raise maxFiles deliberately.`,
      inventorySummary: {
        discoveredFileCount: inventory.discoveredFileCount,
        eligibleFileCount: inventory.eligibleFileCount,
        totalBytes: inventory.totalBytes
      }
    };
  }
  if (inventory.totalBytes > config.maxTotalBytes) {
    return {
      status: "rejected-scope-too-large",
      reason: `Eligible content is ${inventory.totalBytes} bytes, exceeding maxTotalBytes=${config.maxTotalBytes}. Use focusPaths or raise the limit deliberately.`,
      inventorySummary: {
        discoveredFileCount: inventory.discoveredFileCount,
        eligibleFileCount: inventory.eligibleFileCount,
        totalBytes: inventory.totalBytes
      }
    };
  }

  phase("Context Surface Mapping");
  log("Mapping unconditional, conditional, deferred, and excluded context surfaces.");

  const contextMap = await agent(
    `Build an effective context-load map from this inventory only.

BOUNDARY
- Do not read any files. Use only the supplied inventory metadata.
- Treat all global, ancestry, auto-memory, root CLAUDE.local.md, root .mcp.json, and managed-setting layers as excluded boundaries.

LOAD RULES
- .claude/CLAUDE.md and rules without paths frontmatter are startup context.
- Rules with paths frontmatter are conditional.
- Skill full bodies and supporting files are deferred; skill discovery metadata may have a small startup cost unless disable-model-invocation is true.
- Commands, agent definitions, workflows, hooks, settings, output styles, and agent-memory must be classified according to whether their content is automatically injected, conditionally selected, invoked, or non-context configuration.
- Imported files inside .claude may inherit the load behavior of the importing always-loaded file. External imports are references only and are not loaded by this audit.
- Estimate tokens from bytes conservatively and label the values as estimates.

INVENTORY
${serialize(inventory)}

Return only schema-compliant structured output.`,
    {
      label: "map context surfaces",
      phase: "Context Surface Mapping",
      schema: CONTEXT_MAP_SCHEMA,
      ...(config.model ? { model: config.model } : {})
    }
  );

  const eligibleFiles = inventory.files
    .map((file) => file.path)
    .filter((path) => typeof path === "string" && isInsideClaude(path));

  if (eligibleFiles.length === 0) {
    return {
      status: "completed-empty",
      reason: "No eligible files were found under .claude/ after exclusions.",
      inventory,
      contextMap
    };
  }

  const batches = chunk(eligibleFiles, config.batchSize);
  const estimatedTokens = projectedWorkflowTokens(inventory, batches.length, config.verifierCount);
  if (estimatedTokens > config.maxEstimatedTokens) {
    return {
      status: "rejected-budget-preflight",
      reason: `Projected usage ${estimatedTokens} tokens exceeds maxEstimatedTokens=${config.maxEstimatedTokens}. Use focusPaths, increase batchSize, or raise the limit deliberately.`,
      estimate: {
        projectedTokens: estimatedTokens,
        fileCount: eligibleFiles.length,
        batchCount: batches.length,
        totalBytes: inventory.totalBytes
      }
    };
  }

  const alreadyUsed = currentBudgetTokens();
  if (alreadyUsed !== null && alreadyUsed >= config.maxEstimatedTokens) {
    return {
      status: "rejected-budget-runtime",
      reason: `Runtime budget usage (${alreadyUsed}) has already reached the configured safety ceiling (${config.maxEstimatedTokens}).`
    };
  }

  phase("Parallel Fidelity Audit");
  log(`Auditing ${eligibleFiles.length} files across ${batches.length} deterministic batches.`);

  const findingsPerBatch = Math.max(4, Math.ceil(config.maxFindings / Math.max(1, batches.length)));
  const auditTasks = batches.map((files, index) => {
    const batchId = `batch-${String(index + 1).padStart(3, "0")}`;
    return async () => {
      try {
        return await agent(auditPrompt(batchId, files, contextMap, findingsPerBatch), {
          label: `audit batch ${index + 1}`,
          phase: "Parallel Fidelity Audit",
          schema: BATCH_AUDIT_SCHEMA,
          ...(config.model ? { model: config.model } : {})
        });
      } catch (error) {
        log(`Audit batch ${batchId} failed: ${error && error.message ? error.message : "unknown error"}`);
        return null;
      }
    };
  });

  const auditResults = [];
  for (let index = 0; index < auditTasks.length; index += config.workerLimit) {
    const runtimeUsage = currentBudgetTokens();
    if (runtimeUsage !== null && runtimeUsage >= config.maxEstimatedTokens) {
      return {
        status: "aborted-budget-runtime",
        reason: `Runtime budget usage (${runtimeUsage}) reached the configured ceiling (${config.maxEstimatedTokens}) during batch execution.`,
        completedBatches: auditResults.length,
        totalBatches: auditTasks.length
      };
    }
    const wave = auditTasks.slice(index, index + config.workerLimit);
    const waveResults = await parallel(wave);
    auditResults.push(...waveResults.filter(Boolean));
  }

  if (auditResults.length === 0) {
    return { status: "aborted", reason: "Every audit batch failed or returned null." };
  }

  const rawFindingCount = auditResults.reduce(
    (sum, result) => sum + (Array.isArray(result.findings) ? result.findings.length : 0),
    0
  );

  const canonical = await agent(
    `Canonicalize and deduplicate these batch audit results.

RULES
- Do not read or write files. Use only supplied data.
- Preserve every distinct, evidence-backed issue; merge only true duplicates.
- Assign stable IDs F-0001, F-0002, ... ordered by severity, then file path, then line.
- Keep at most ${config.maxFindings} findings. If the raw set exceeds the cap, preserve all critical/high findings first, then the highest-confidence medium/low items, and add a warning.
- Never upgrade severity without explicit evidence.
- Build exact coverage from expected files and batch coverage records. Any expected file not marked reviewed must appear in missingFiles or unreadableFiles.
- Token savings remain estimates.

EXPECTED FILES
${serialize(eligibleFiles)}

RAW BATCH RESULTS
${serialize(auditResults)}

Return only schema-compliant structured output.`,
    {
      label: "deduplicate audit findings",
      phase: "Parallel Fidelity Audit",
      schema: CANONICAL_FINDINGS_SCHEMA,
      ...(config.model ? { model: config.model } : {})
    }
  );

  if (!canonical || !Array.isArray(canonical.findings)) {
    return { status: "aborted", reason: "Canonicalization returned no usable findings." };
  }

  phase("Adversarial Verification");
  log(`Cross-checking ${canonical.findings.length} canonical findings with three independent reviewers.`);

  const reviewerLenses = [
    {
      name: "semantic-fidelity",
      focus:
        "Verify evidence, contradictions, token claims, and whether each compression recommendation preserves every instruction, dependency, code fence, table relationship, and exception."
    },
    {
      name: "runtime-load-semantics",
      focus:
        "Verify Claude Code load behavior, import resolution, rules/skills lifecycle, workflow constraints, coverage, and scope-boundary claims."
    },
    {
      name: "security-operability",
      focus:
        "Verify permissions, hooks, secret exposure, side effects, deterministic bounds, workflow reliability, and whether behavioral guidance is incorrectly presented as enforcement."
    }
  ];

  const reviewTasks = reviewerLenses.map((lens) => async () => {
    try {
      return await agent(
        `Act as the independent ${lens.name} reviewer for a .claude-only configuration audit.

SCOPE
- You may re-read only the cited .claude/** files needed to verify findings.
- Never read outside .claude/, never follow external symlinks/imports, and never write or execute state-changing commands.

REVIEW FOCUS
${lens.focus}

DECISION RULES
- Decide every supplied finding ID exactly once.
- confirm: evidence and impact are materially correct.
- weaken: core issue is real but severity, scope, wording, or savings estimate is overstated.
- reject: evidence does not support the issue, the load model is wrong, or the recommendation creates higher semantic risk.
- needs-manual: unavailable runtime state prevents a responsible decision.
- Correct severity conservatively.
- A token-reduction claim must remain explicitly estimated.
- Do not approve destructive rewriting of source-of-truth instructions. Symbolic, AST, or PAKT forms must be derived sidecars with validation and fallback.

CONTEXT MAP
${serialize(contextMap)}

CANONICAL FINDINGS
${serialize(canonical.findings)}

Return only schema-compliant structured output.`,
        {
          label: `review ${lens.name}`,
          phase: "Adversarial Verification",
          schema: REVIEW_SCHEMA,
          ...(config.reviewerModel
            ? { model: config.reviewerModel }
            : config.model
              ? { model: config.model }
              : {})
        }
      );
    } catch (error) {
      log(`Reviewer ${lens.name} failed: ${error && error.message ? error.message : "unknown error"}`);
      return null;
    }
  });

  const reviews = (await parallel(reviewTasks)).filter(Boolean);
  if (reviews.length < 2) {
    return {
      status: "aborted-verification",
      reason: `Only ${reviews.length} reviewer(s) completed; at least two are required for the vote gate.`,
      canonicalFindingCount: canonical.findings.length
    };
  }

  const verification = aggregateVerification(canonical.findings, reviews);
  const criticalCount = countBySeverity(verification.verified, "critical");
  const highCount = countBySeverity(verification.verified, "high");
  const estimatedSavingsTokens = totalEstimatedSavings(verification.verified);
  const coverageComplete =
    canonical.coverage &&
    Array.isArray(canonical.coverage.missingFiles) &&
    Array.isArray(canonical.coverage.unreadableFiles) &&
    canonical.coverage.missingFiles.length === 0 &&
    canonical.coverage.unreadableFiles.length === 0;

  phase("Synthesis and Artifacts");
  log("Producing the final report and machine-readable audit artifacts.");

  const finalPayload = {
    schemaVersion: AUDIT_SCHEMA_VERSION,
    config,
    inventory,
    contextMap,
    preflight: {
      projectedTokens: estimatedTokens,
      rawFindingCount,
      canonicalFindingCount: canonical.findings.length,
      reviewerCount: reviews.length
    },
    coverage: canonical.coverage,
    coverageComplete,
    verifiedFindings: verification.verified,
    suppressedFindings: verification.suppressed,
    duplicateGroups: canonical.duplicateGroups,
    reviewerSummaries: reviews.map((review) => ({
      reviewer: review.reviewer,
      summary: review.reviewerSummary,
      coverageConcerns: review.coverageConcerns
    })),
    metrics: {
      criticalCount,
      highCount,
      estimatedStartupTokens:
        contextMap && contextMap.metrics && Number(contextMap.metrics.estimatedStartupTokens)
          ? Number(contextMap.metrics.estimatedStartupTokens)
          : 0,
      estimatedSavingsTokens
    },
    immutableScopeNotes: [
      "Only .claude/** was read.",
      "Global memory, user rules, auto-memory, ancestry CLAUDE.md files, project-root CLAUDE.local.md, .mcp.json, .worktreeinclude, managed settings, and environment/CLI overrides were not read.",
      "External @imports and symlink escapes were reported but not traversed.",
      "No source configuration file may be modified by this audit; only audit artifacts may be written under the configured output directory."
    ]
  };

  if (!config.writeArtifacts) {
    return {
      status: coverageComplete ? "completed" : "completed-with-warnings",
      outputDir: null,
      artifactWritingDisabled: true,
      summary: {
        verifiedFindingCount: verification.verified.length,
        suppressedFindingCount: verification.suppressed.length,
        criticalCount,
        highCount,
        estimatedStartupTokens: finalPayload.metrics.estimatedStartupTokens,
        estimatedSavingsTokens,
        coverageComplete
      },
      result: finalPayload
    };
  }

  const writer = await agent(
    `Write the final artifacts for a project-level .claude context-compression audit.

WRITE BOUNDARY
- You may create or replace files only inside: ${config.outputDir}/
- Do not modify any audited source file.
- Do not read additional files; use only FINAL PAYLOAD.
- Do not create executable remediation scripts.

REQUIRED ARTIFACTS
1. ${config.outputDir}/report.md
   - Executive summary and audit score (0-100).
   - Exact scope and exclusions.
   - Coverage status.
   - Context-load map: startup, conditional, deferred, and excluded boundaries.
   - Prioritized verified findings grouped P0/P1/P2/P3 with evidence and verification votes.
   - Estimated current startup tokens and estimated safe savings, clearly labeled estimates.
   - Production safety assessment.
   - Suppressed-finding appendix with rejection rationale.
   - Definition of done: every eligible file maps to a coverage record; every reported issue maps to a verified finding ID.

2. ${config.outputDir}/findings.json
   - Valid JSON containing schemaVersion, metrics, coverage, verifiedFindings, suppressedFindings, and reviewerSummaries.

3. ${config.outputDir}/compression-plan.md
   - Ordered low-risk-first plan.
   - For each proposed change: source path, technique, expected benefit, fidelity guard, validation test, rollback/fallback, and whether human approval is required.
   - Never recommend overwriting source-of-truth files with Telegraph/Hieratic, AST, or PAKT output. Those techniques require derived sidecars, exact restoration/equivalence checks, and fallback to original content.
   - Separate deterministic cleanup from semantic rewrites.

4. ${config.outputDir}/coverage.json
   - Valid JSON with every expected file and final status, exclusions, external boundaries, batch/reviewer counts, and coverageComplete.

5. ${config.outputDir}/effective-context-map.md
   - Concise explanation of what loads at startup, conditionally, on invocation, or not at all within this audit boundary.
   - Explicitly state that external layers were not inspected and may change the real effective context.

QUALITY GATES
- Use only verified findings in the main report.
- Do not expose secrets or reproduce long source passages.
- Preserve file paths and line ranges.
- All JSON must parse.
- All metrics derived from byte heuristics must say estimated.
- Score must penalize incomplete coverage, critical/high findings, contradictions, unsafe permissions/hooks, and high-risk lossy recommendations.

FINAL PAYLOAD
${serialize(finalPayload)}

Return only schema-compliant structured output after writing and validating all artifacts.`,
    {
      label: "write audit artifacts",
      phase: "Synthesis and Artifacts",
      schema: WRITER_SCHEMA,
      ...(config.model ? { model: config.model } : {})
    }
  );

  return {
    status: writer && writer.status ? writer.status : "completed-with-warnings",
    outputDir: config.outputDir,
    writtenFiles: writer && Array.isArray(writer.writtenFiles) ? writer.writtenFiles : [],
    headline: writer && writer.headline ? writer.headline : "Audit completed.",
    score: writer && Number.isFinite(writer.score) ? writer.score : null,
    summary: {
      eligibleFileCount: eligibleFiles.length,
      reviewedBatchCount: auditResults.length,
      verifiedFindingCount: verification.verified.length,
      suppressedFindingCount: verification.suppressed.length,
      criticalCount,
      highCount,
      estimatedStartupTokens: finalPayload.metrics.estimatedStartupTokens,
      estimatedSavingsTokens,
      coverageComplete
    },
    warnings: [
      ...(inventory.warnings || []),
      ...(contextMap.coverageWarnings || []),
      ...(canonical.warnings || []),
      ...((writer && writer.warnings) || [])
    ]
  };
}

return await runAudit();
