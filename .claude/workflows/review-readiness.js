// review-readiness.js

export const meta = {
  name: "review_readiness",
  description:
    "Reviews a PR, branch, commit range, or local diff for merge readiness using findings-first risk analysis and independent verification gates.",
  whenToUse:
    "Use before merging a pull request or local diff when you need a blocker-first verdict, release evidence review, and explicit unknowns.",
  phases: [
    {
      title: "Input Gate",
      detail: "Normalize review scope, runtime limits, and verification expectations."
    },
    {
      title: "Context Discovery",
      detail: "Inspect the target diff/PR and summarize changed files, intent, and available evidence."
    },
    {
      title: "Scale & Budget Gate",
      detail: "Abort or narrow scope before expensive review if the change set is too large."
    },
    {
      title: "Parallel Risk Review",
      detail: "Fan out focused reviewers across behavior, privacy, provider, storage, and release evidence surfaces."
    },
    {
      title: "Adversarial Verification",
      detail: "Challenge unsupported blockers, optimistic approvals, and missing verification requirements."
    },
    {
      title: "Merge Readiness Synthesis",
      detail: "Produce a verdict with blockers, suggestions, verification gaps, and safe next actions."
    }
  ],
  productionConfig: {
    maxChangedFilesDefault: 80,
    maxReviewWorkUnitsDefault: 420,
    maxBudgetTokensDefault: 2500000,
    validationRequirements: [
      "blocking findings must include concrete evidence",
      "approval requires inspected verification evidence",
      "unknowns must remain verification gaps, not approvals",
      "privacy, provider, storage, and release risks must be explicitly considered"
    ],
    riskSurfaces: [
      "incorrect behavior changes",
      "privacy or secret leakage",
      "provider, routing, or streaming drift",
      "storage, migration, or recovery regressions",
      "release evidence gaps"
    ]
  }
};

const DEFAULT_WORKER_MODEL = "claude-sonnet-4-6";
const DEFAULT_REVIEW_MODEL = "claude-opus-4-8";

const REVIEW_LENSES = [
  {
    id: "behavior",
    title: "Behavior and regression risk",
    checklist: [
      "changed behavior matches stated intent",
      "edge cases and error paths are preserved",
      "tests or manual verification cover the risky paths",
      "no accidental API, type, or contract drift"
    ],
    suggestedSkills: ["repo-audit"]
  },
  {
    id: "privacy",
    title: "Privacy, secrets, and data boundary risk",
    checklist: [
      "no secrets, tokens, credentials, or private identifiers are exposed",
      "new logging/telemetry avoids sensitive payloads",
      "data retention and access boundaries remain intact",
      "user-visible privacy behavior is not broadened without evidence"
    ],
    suggestedSkills: ["privacy-boundary-review"]
  },
  {
    id: "provider-routing",
    title: "Provider, routing, and streaming drift",
    checklist: [
      "model/provider routing behavior remains intentional",
      "streaming, retry, timeout, and fallback semantics are preserved",
      "provider-specific payloads are validated",
      "no silent degradation in multi-provider behavior"
    ],
    suggestedSkills: ["provider-routing-review"]
  },
  {
    id: "storage-recovery",
    title: "Storage, migration, and recovery risk",
    checklist: [
      "schema/storage changes have migration and rollback evidence",
      "reads/writes preserve compatibility with existing data",
      "recovery paths and idempotency are considered",
      "parallel or retry behavior cannot corrupt state"
    ],
    suggestedSkills: ["storage-recovery-review"]
  },
  {
    id: "release-evidence",
    title: "Release evidence and operational readiness",
    checklist: [
      "relevant tests/build/lint/typecheck results are present",
      "deployment, config, and environment assumptions are documented",
      "monitoring, feature flag, or rollback plan exists when risk warrants it",
      "release notes or customer impact are clear when applicable"
    ],
    suggestedSkills: ["release-evidence-review"]
  }
];

const changedFileSchema = {
  type: "object",
  properties: {
    path: { type: "string" },
    changeType: {
      type: "string",
      enum: ["added", "modified", "deleted", "renamed", "copied", "unknown"]
    },
    riskHints: {
      type: "array",
      items: { type: "string" }
    },
    reasonToInspect: { type: "string" }
  },
  required: ["path", "changeType", "riskHints", "reasonToInspect"]
};

const verificationEvidenceSchema = {
  type: "object",
  properties: {
    kind: {
      type: "string",
      enum: [
        "test",
        "build",
        "lint",
        "typecheck",
        "manual",
        "screenshot",
        "log",
        "ci",
        "migration",
        "rollback",
        "unknown"
      ]
    },
    commandOrSource: { type: "string" },
    status: {
      type: "string",
      enum: ["passed", "failed", "missing", "not-applicable", "unknown"]
    },
    evidence: { type: "string" }
  },
  required: ["kind", "commandOrSource", "status", "evidence"]
};

const reviewContextSchema = {
  type: "object",
  properties: {
    normalizedScope: { type: "string" },
    scopeType: {
      type: "string",
      enum: ["pull-request", "local-diff", "branch", "commit-range", "unknown"]
    },
    summary: { type: "string" },
    statedIntent: { type: "string" },
    changedFiles: {
      type: "array",
      items: changedFileSchema
    },
    highestRiskAreas: {
      type: "array",
      items: { type: "string" }
    },
    availableVerification: {
      type: "array",
      items: verificationEvidenceSchema
    },
    explicitUnknowns: {
      type: "array",
      items: { type: "string" }
    },
    recommendedReviewFocus: {
      type: "array",
      items: { type: "string" }
    }
  },
  required: [
    "normalizedScope",
    "scopeType",
    "summary",
    "statedIntent",
    "changedFiles",
    "highestRiskAreas",
    "availableVerification",
    "explicitUnknowns",
    "recommendedReviewFocus"
  ]
};

const findingSchema = {
  type: "object",
  properties: {
    findingId: { type: "string" },
    title: { type: "string" },
    severity: {
      type: "string",
      enum: ["critical", "high", "medium", "low"]
    },
    blocking: { type: "boolean" },
    filePath: { type: "string" },
    lineOrSymbol: { type: "string" },
    evidence: { type: "string" },
    impact: { type: "string" },
    requiredAction: { type: "string" },
    confidence: {
      type: "string",
      enum: ["high", "medium", "low"]
    }
  },
  required: [
    "title",
    "severity",
    "blocking",
    "filePath",
    "lineOrSymbol",
    "evidence",
    "impact",
    "requiredAction",
    "confidence"
  ]
};

const verificationGapSchema = {
  type: "object",
  properties: {
    gap: { type: "string" },
    riskIfUnverified: { type: "string" },
    recommendedCommandOrEvidence: { type: "string" },
    blocksMerge: { type: "boolean" }
  },
  required: ["gap", "riskIfUnverified", "recommendedCommandOrEvidence", "blocksMerge"]
};

const lensReportSchema = {
  type: "object",
  properties: {
    lens: { type: "string" },
    inspectedScope: { type: "string" },
    blockingFindings: {
      type: "array",
      items: findingSchema
    },
    nonBlockingSuggestions: {
      type: "array",
      items: findingSchema
    },
    verificationGaps: {
      type: "array",
      items: verificationGapSchema
    },
    evidenceReviewed: {
      type: "array",
      items: { type: "string" }
    },
    confidence: {
      type: "string",
      enum: ["high", "medium", "low"]
    },
    summary: { type: "string" }
  },
  required: [
    "lens",
    "inspectedScope",
    "blockingFindings",
    "nonBlockingSuggestions",
    "verificationGaps",
    "evidenceReviewed",
    "confidence",
    "summary"
  ]
};

const adversarialReportSchema = {
  type: "object",
  properties: {
    reviewerRole: { type: "string" },
    supportedBlockerIds: {
      type: "array",
      items: { type: "string" }
    },
    unsupportedBlockers: {
      type: "array",
      items: {
        type: "object",
        properties: {
          findingId: { type: "string" },
          reason: { type: "string" }
        },
        required: ["findingId", "reason"]
      }
    },
    missedBlockingRisks: {
      type: "array",
      items: findingSchema
    },
    additionalVerificationGaps: {
      type: "array",
      items: verificationGapSchema
    },
    verdictConcern: {
      type: "string",
      enum: ["none", "caution", "veto"]
    },
    summary: { type: "string" }
  },
  required: [
    "reviewerRole",
    "supportedBlockerIds",
    "unsupportedBlockers",
    "missedBlockingRisks",
    "additionalVerificationGaps",
    "verdictConcern",
    "summary"
  ]
};

const readinessReportSchema = {
  type: "object",
  properties: {
    status: {
      type: "string",
      enum: ["ready", "ready-with-cautions", "not-ready", "needs-more-evidence", "aborted"]
    },
    verdict: { type: "string" },
    blockingFindings: {
      type: "array",
      items: findingSchema
    },
    nonBlockingSuggestions: {
      type: "array",
      items: findingSchema
    },
    verificationGaps: {
      type: "array",
      items: verificationGapSchema
    },
    mergeSafetySummary: { type: "string" },
    releaseRiskSummary: { type: "string" },
    requiredNextActions: {
      type: "array",
      items: { type: "string" }
    },
    evidenceReviewed: {
      type: "array",
      items: { type: "string" }
    },
    explicitUnknowns: {
      type: "array",
      items: { type: "string" }
    },
    outputMarkdown: { type: "string" }
  },
  required: [
    "status",
    "verdict",
    "blockingFindings",
    "nonBlockingSuggestions",
    "verificationGaps",
    "mergeSafetySummary",
    "releaseRiskSummary",
    "requiredNextActions",
    "evidenceReviewed",
    "explicitUnknowns",
    "outputMarkdown"
  ]
};

function runtimeArgs(explicitArgs) {
  if (typeof explicitArgs !== "undefined") return explicitArgs;
  if (typeof args !== "undefined") return args;
  return {};
}

function positiveInteger(value, fallback, minValue, maxValue) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return fallback;
  const integer = numeric < 0 ? Math.ceil(numeric) : Math.floor(numeric);
  if (integer < minValue) return minValue;
  if (integer > maxValue) return maxValue;
  return integer;
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function firstPresent(values) {
  for (const value of values) {
    if (typeof value === "string" && value.trim()) return value.trim();
    if (typeof value === "number") return String(value);
  }
  return "";
}

function normalizeConfig(explicitArgs) {
  const source = runtimeArgs(explicitArgs);

  if (typeof source === "string") {
    return {
      scope: source.trim(),
      baseBranch: "main",
      targetBranch: "",
      verificationCommands: [],
      dryRun: false,
      strictMode: true,
      maxChangedFiles: meta.productionConfig.maxChangedFilesDefault,
      maxReviewWorkUnits: meta.productionConfig.maxReviewWorkUnitsDefault,
      maxBudgetTokens: meta.productionConfig.maxBudgetTokensDefault,
      workerModel: DEFAULT_WORKER_MODEL,
      reviewModel: DEFAULT_REVIEW_MODEL
    };
  }

  const safeSource = source && typeof source === "object" ? source : {};
  const scope = firstPresent([
    safeSource.scope,
    safeSource.prUrl,
    safeSource.pr,
    safeSource.prNumber,
    safeSource.diffScope,
    safeSource.target,
    safeSource.branch,
    safeSource.commitRange
  ]);

  return {
    scope,
    baseBranch: firstPresent([safeSource.baseBranch, safeSource.base]) || "main",
    targetBranch: firstPresent([safeSource.targetBranch, safeSource.head]) || "",
    verificationCommands: asArray(safeSource.verificationCommands),
    dryRun: safeSource.dryRun === true,
    strictMode: safeSource.strictMode !== false,
    maxChangedFiles: positiveInteger(
      safeSource.maxChangedFiles,
      meta.productionConfig.maxChangedFilesDefault,
      1,
      500
    ),
    maxReviewWorkUnits: positiveInteger(
      safeSource.maxReviewWorkUnits,
      meta.productionConfig.maxReviewWorkUnitsDefault,
      1,
      3000
    ),
    maxBudgetTokens: positiveInteger(
      safeSource.maxBudgetTokens,
      meta.productionConfig.maxBudgetTokensDefault,
      50000,
      20000000
    ),
    workerModel: firstPresent([safeSource.workerModel]) || DEFAULT_WORKER_MODEL,
    reviewModel: firstPresent([safeSource.reviewModel]) || DEFAULT_REVIEW_MODEL
  };
}

function emitPhase(title) {
  if (typeof phase === "function") phase(title);
}

function emitLog(message) {
  if (typeof log === "function") log(message);
}

function stringifyForPrompt(value, maxChars) {
  const rendered = typeof value === "string" ? value : JSON.stringify(value, null, 2);
  if (rendered.length <= maxChars) return rendered;
  return `${rendered.slice(0, maxChars)}\n...[truncated by workflow; inspect source directly for omitted detail]`;
}

function coerceObject(value, fallback) {
  if (!value) return fallback;
  if (typeof value === "object") return value;
  if (typeof value === "string") {
    try {
      return JSON.parse(value);
    } catch (error) {
      return fallback;
    }
  }
  return fallback;
}

function observedBudgetTokens() {
  if (typeof budget === "undefined" || !budget) return 0;
  if (typeof budget.total === "number") return budget.total;
  if (budget.tokens && typeof budget.tokens.total === "number") return budget.tokens.total;
  if (budget.usage && typeof budget.usage.total === "number") return budget.usage.total;
  return 0;
}

function tooMuchBudgetUsed(config) {
  const total = observedBudgetTokens();
  return total > 0 && total >= config.maxBudgetTokens;
}

async function runParallel(thunks) {
  if (typeof parallel === "function") return await parallel(thunks);

  const results = [];
  for (const thunk of thunks) {
    try {
      results.push(await thunk());
    } catch (error) {
      results.push(null);
    }
  }
  return results;
}

async function runAgent(prompt, options, config, mockValue) {
  const label = options && options.label ? options.label : "unlabeled";

  if (config.dryRun) {
    emitLog(`[dry-run] ${label}`);
    return mockValue;
  }

  if (typeof agent !== "function") {
    throw new Error("agent() is unavailable. Run with dryRun: true for local orchestration checks.");
  }

  try {
    return await agent(prompt, options);
  } catch (error) {
    const message = error && error.message ? error.message : String(error);
    emitLog(`[agent-error] ${label}: ${message}`);
    return null;
  }
}

function buildContextPrompt(config) {
  return `You are the context discovery agent for the review_readiness workflow.

Scope to review: ${config.scope}
Base branch, when applicable: ${config.baseBranch}
Target branch, when applicable: ${config.targetBranch || "not provided"}
Expected verification commands or evidence supplied by user:
${stringifyForPrompt(config.verificationCommands, 4000)}

Instructions:
1. Use read-only inspection only. Do not modify files.
2. If this is a PR URL or PR number, inspect the PR metadata, diff, changed files, CI/test evidence, and stated intent. Use available repository tooling if configured.
3. If this is a local scope, inspect the relevant git diff, changed files, commit range, or branch comparison.
4. Identify changed files and the highest-risk areas. Keep unknowns explicit.
5. Do not approve the change. Only return context for downstream reviewers.

Return structured data matching the provided schema.`;
}

function buildLensPrompt(lens, config, context) {
  return `You are an independent merge-readiness reviewer focused only on: ${lens.title}.

Suggested review checklists/skills: ${lens.suggestedSkills.join(", ")}
Review checklist:
- ${lens.checklist.join("\n- ")}

Review scope: ${config.scope}
Strict mode: ${config.strictMode ? "enabled" : "disabled"}

Context discovered by the workflow:
${stringifyForPrompt(context, 12000)}

Instructions:
1. Inspect the repository, diff, PR, or local changes directly where needed. Do not rely only on the context summary.
2. Report only concrete blocker findings when there is specific evidence. Every blocker must include file/symbol evidence, impact, and required action.
3. Put uncertain but important missing evidence in verificationGaps, not in blockingFindings unless the missing evidence itself makes merge unsafe.
4. Separate non-blocking improvements from true merge blockers.
5. Unknowns must remain unknown; never soften missing evidence into approval.
6. Do not edit files.

Return structured data matching the provided schema.`;
}

function buildAdversarialPrompt(role, config, context, lensReports) {
  return `You are an adversarial verification reviewer for review_readiness.

Reviewer role: ${role}
Review scope: ${config.scope}
Strict mode: ${config.strictMode ? "enabled" : "disabled"}

Context:
${stringifyForPrompt(context, 9000)}

Preliminary lens reports:
${stringifyForPrompt(lensReports, 16000)}

Instructions:
1. Challenge both false positives and false approvals.
2. Mark a blocker as supported only when evidence and impact are concrete.
3. Mark a blocker as unsupported when it is speculative, lacks file/symbol evidence, or should be downgraded to a verification gap.
4. Add missedBlockingRisks only when you can point to concrete evidence.
5. Add verification gaps when evidence is missing for a risky change path.
6. Do not edit files.

Return structured data matching the provided schema.`;
}

function buildSynthesisPrompt(config, context, lensReports, adversarialReports) {
  return `You are the final merge-readiness synthesizer.

Review scope: ${config.scope}
Strict mode: ${config.strictMode ? "enabled" : "disabled"}

Decision rules:
- status "not-ready" when supported blocking findings remain.
- status "needs-more-evidence" when no concrete blocker is proven but required verification is missing for risky behavior.
- status "ready-with-cautions" when no blockers remain and verification is adequate, but low-risk follow-ups exist.
- status "ready" only when no blockers remain, verification evidence is adequate, and unknowns are not material.
- Never claim tests passed unless evidence says they passed.
- Unsupported blockers should not appear as final blockers; convert them to suggestions or verification gaps as appropriate.

Context:
${stringifyForPrompt(context, 10000)}

Lens reports:
${stringifyForPrompt(lensReports, 18000)}

Adversarial verification reports:
${stringifyForPrompt(adversarialReports, 12000)}

Required output sections in outputMarkdown:
1. verdict
2. blocking-findings
3. non-blocking-suggestions
4. verification-gaps
5. merge-safety-summary

Return structured data matching the provided schema.`;
}

function mockContext(config) {
  return {
    normalizedScope: config.scope || "dry-run-scope",
    scopeType: "unknown",
    summary: "Dry-run context. No live repository or PR inspection was performed.",
    statedIntent: "Validate workflow orchestration without live agent calls.",
    changedFiles: [
      {
        path: "src/example.js",
        changeType: "modified",
        riskHints: ["behavior"],
        reasonToInspect: "Mock changed file used for dry-run control-flow validation."
      }
    ],
    highestRiskAreas: ["release evidence gaps"],
    availableVerification: [
      {
        kind: "unknown",
        commandOrSource: "dry-run",
        status: "unknown",
        evidence: "Dry-run mode bypassed live verification."
      }
    ],
    explicitUnknowns: ["Live diff was not inspected in dry-run mode."],
    recommendedReviewFocus: ["Validate live run against the actual PR or diff scope."]
  };
}

function mockLensReport(lens) {
  return {
    lens: lens.id,
    inspectedScope: "dry-run",
    blockingFindings: [],
    nonBlockingSuggestions: [
      {
        findingId: `${lens.id}-suggestion-1`,
        title: "Dry-run suggestion placeholder",
        severity: "low",
        blocking: false,
        filePath: "src/example.js",
        lineOrSymbol: "n/a",
        evidence: "Dry-run mock data only.",
        impact: "No live impact assessed.",
        requiredAction: "Run without dryRun to inspect the real change.",
        confidence: "low"
      }
    ],
    verificationGaps: [
      {
        gap: "Live verification evidence not inspected in dry-run mode.",
        riskIfUnverified: "A real merge-readiness verdict cannot be issued from mock data.",
        recommendedCommandOrEvidence: "Run workflow with dryRun false against a PR, branch, or diff.",
        blocksMerge: true
      }
    ],
    evidenceReviewed: ["dry-run mock data"],
    confidence: "low",
    summary: `Dry-run ${lens.id} lens completed.`
  };
}

function mockAdversarialReport(role) {
  return {
    reviewerRole: role,
    supportedBlockerIds: [],
    unsupportedBlockers: [],
    missedBlockingRisks: [],
    additionalVerificationGaps: [
      {
        gap: "Dry-run cannot verify real blockers or approval safety.",
        riskIfUnverified: "Workflow control flow may pass while the real change still has issues.",
        recommendedCommandOrEvidence: "Execute a live review with repository access.",
        blocksMerge: true
      }
    ],
    verdictConcern: "caution",
    summary: "Dry-run adversarial verification completed with no live evidence."
  };
}

function mockSynthesis() {
  return {
    status: "needs-more-evidence",
    verdict: "Dry-run completed. Live review required before merge.",
    blockingFindings: [],
    nonBlockingSuggestions: [],
    verificationGaps: [
      {
        gap: "No live PR/diff evidence was inspected.",
        riskIfUnverified: "Cannot determine merge safety.",
        recommendedCommandOrEvidence: "Run without dryRun against the target scope.",
        blocksMerge: true
      }
    ],
    mergeSafetySummary: "The workflow logic executed, but no real merge-readiness verdict is possible in dry-run mode.",
    releaseRiskSummary: "Release risk unknown until live evidence is inspected.",
    requiredNextActions: ["Run the workflow against the actual PR, branch, commit range, or local diff."],
    evidenceReviewed: ["dry-run mock data"],
    explicitUnknowns: ["Actual diff, tests, CI, and release evidence were not inspected."],
    outputMarkdown:
      "## verdict\nneeds-more-evidence — dry-run only.\n\n## blocking-findings\nNone from mock data.\n\n## non-blocking-suggestions\nRun a live review.\n\n## verification-gaps\nNo live evidence inspected.\n\n## merge-safety-summary\nDo not merge based on dry-run output."
  };
}

function assignFindingIds(lensReports) {
  return asArray(lensReports).map((report, reportIndex) => {
    const safeReport = coerceObject(report, {
      lens: `unknown-${reportIndex + 1}`,
      blockingFindings: [],
      nonBlockingSuggestions: [],
      verificationGaps: [],
      evidenceReviewed: [],
      confidence: "low",
      summary: "Reviewer returned no usable report."
    });

    const lensId = safeReport.lens || `lens-${reportIndex + 1}`;

    const blockers = asArray(safeReport.blockingFindings).map((finding, index) => {
      const safeFinding = coerceObject(finding, {});
      if (!safeFinding.findingId) safeFinding.findingId = `${lensId}-blocker-${index + 1}`;
      safeFinding.blocking = true;
      return safeFinding;
    });

    const suggestions = asArray(safeReport.nonBlockingSuggestions).map((finding, index) => {
      const safeFinding = coerceObject(finding, {});
      if (!safeFinding.findingId) safeFinding.findingId = `${lensId}-suggestion-${index + 1}`;
      safeFinding.blocking = false;
      return safeFinding;
    });

    safeReport.blockingFindings = blockers;
    safeReport.nonBlockingSuggestions = suggestions;
    safeReport.verificationGaps = asArray(safeReport.verificationGaps);
    safeReport.evidenceReviewed = asArray(safeReport.evidenceReviewed);
    return safeReport;
  });
}

function aborted(status, verdict, details) {
  return {
    status,
    verdict,
    blockingFindings: [],
    nonBlockingSuggestions: [],
    verificationGaps: [],
    mergeSafetySummary: details,
    releaseRiskSummary: "Release risk was not assessed because the workflow stopped before review completion.",
    requiredNextActions: [details],
    evidenceReviewed: [],
    explicitUnknowns: [details],
    outputMarkdown: `## verdict\n${status} — ${verdict}\n\n## merge-safety-summary\n${details}`
  };
}

export default async function runWorkflow(inputArgs) {
  const config = normalizeConfig(inputArgs);

  emitPhase("Input Gate");

  if (!config.scope) {
    return aborted(
      "aborted",
      "Missing review scope.",
      "Provide args.scope, args.prUrl, args.prNumber, args.diffScope, args.branch, or args.commitRange."
    );
  }

  if (tooMuchBudgetUsed(config)) {
    return aborted(
      "aborted",
      "Budget gate blocked review before discovery.",
      `Observed token usage already meets or exceeds configured maxBudgetTokens (${config.maxBudgetTokens}).`
    );
  }

  emitLog(`Review scope: ${config.scope}`);
  emitLog(
    `Limits: maxChangedFiles=${config.maxChangedFiles}, maxReviewWorkUnits=${config.maxReviewWorkUnits}, maxBudgetTokens=${config.maxBudgetTokens}`
  );

  emitPhase("Context Discovery");
  const rawContext = await runAgent(
    buildContextPrompt(config),
    {
      label: "discover context",
      model: config.workerModel,
      schema: reviewContextSchema
    },
    config,
    mockContext(config)
  );

  const context = coerceObject(rawContext, null);
  if (!context) {
    return aborted(
      "aborted",
      "Context discovery failed.",
      "The workflow could not inspect the target scope. Confirm the PR/diff exists and repository tools are available."
    );
  }

  const changedFiles = asArray(context.changedFiles);

  emitPhase("Scale & Budget Gate");
  const reviewWorkUnits = changedFiles.length * REVIEW_LENSES.length;

  if (changedFiles.length === 0) {
    return aborted(
      "needs-more-evidence",
      "No changed files were discovered.",
      "No diff files were identified. Provide a narrower or valid PR/diff scope, or confirm this is a metadata-only review."
    );
  }

  if (changedFiles.length > config.maxChangedFiles) {
    return aborted(
      "aborted",
      "Change set exceeds review-readiness safety limit.",
      `Discovered ${changedFiles.length} changed files, above maxChangedFiles=${config.maxChangedFiles}. Narrow the scope or raise the limit intentionally.`
    );
  }

  if (reviewWorkUnits > config.maxReviewWorkUnits) {
    return aborted(
      "aborted",
      "Projected review work exceeds configured safety limit.",
      `Projected ${reviewWorkUnits} file-lens work units, above maxReviewWorkUnits=${config.maxReviewWorkUnits}. Narrow the scope or raise the limit intentionally.`
    );
  }

  if (tooMuchBudgetUsed(config)) {
    return aborted(
      "aborted",
      "Budget gate blocked review after discovery.",
      `Observed token usage meets or exceeds configured maxBudgetTokens (${config.maxBudgetTokens}).`
    );
  }

  emitLog(
    `Discovered ${changedFiles.length} changed files; launching ${REVIEW_LENSES.length} focused review lenses.`
  );

  emitPhase("Parallel Risk Review");
  const lensTasks = REVIEW_LENSES.map((lens) => async () => {
    return await runAgent(
      buildLensPrompt(lens, config, context),
      {
        label: `review ${lens.id}`,
        model: config.workerModel,
        schema: lensReportSchema
      },
      config,
      mockLensReport(lens)
    );
  });

  const rawLensReports = await runParallel(lensTasks);
  const lensReports = assignFindingIds(rawLensReports.filter(Boolean));

  if (lensReports.length === 0) {
    return aborted(
      "aborted",
      "All parallel review lenses failed.",
      "No usable review reports were returned. Re-run with a narrower scope or inspect repository/tool access."
    );
  }

  if (tooMuchBudgetUsed(config)) {
    return aborted(
      "aborted",
      "Budget gate blocked adversarial verification.",
      `Observed token usage meets or exceeds configured maxBudgetTokens (${config.maxBudgetTokens}).`
    );
  }

  emitPhase("Adversarial Verification");
  const adversarialRoles = [
    "blocker evidence skeptic",
    "optimistic approval challenger",
    "verification gap auditor"
  ];

  const adversarialTasks = adversarialRoles.map((role) => async () => {
    return await runAgent(
      buildAdversarialPrompt(role, config, context, lensReports),
      {
        label: role,
        model: config.reviewModel,
        schema: adversarialReportSchema
      },
      config,
      mockAdversarialReport(role)
    );
  });

  const rawAdversarialReports = await runParallel(adversarialTasks);
  const adversarialReports = rawAdversarialReports
    .filter(Boolean)
    .map((report, index) =>
      coerceObject(report, {
        reviewerRole: adversarialRoles[index] || `reviewer-${index + 1}`,
        supportedBlockerIds: [],
        unsupportedBlockers: [],
        missedBlockingRisks: [],
        additionalVerificationGaps: [],
        verdictConcern: "caution",
        summary: "No usable adversarial report was returned."
      })
    );

  emitPhase("Merge Readiness Synthesis");
  const rawSynthesis = await runAgent(
    buildSynthesisPrompt(config, context, lensReports, adversarialReports),
    {
      label: "synthesize verdict",
      model: config.reviewModel,
      schema: readinessReportSchema
    },
    config,
    mockSynthesis()
  );

  const synthesis = coerceObject(rawSynthesis, null);
  if (!synthesis) {
    return aborted(
      "aborted",
      "Synthesis failed.",
      "Risk lenses completed, but the final readiness report could not be generated. Inspect raw lens outputs and re-run synthesis."
    );
  }

  return synthesis;
}