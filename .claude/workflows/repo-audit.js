export const meta = {
  name: "repo-audit",
  description:
    "Evidence-led repository audit with stack classification, parallel boundary review, adversarial verification, and final risk synthesis.",
  whenToUse:
    "Run before releases, after provider/storage/privacy boundary changes, or when validating a desktop client scaffold against security and reliability contracts.",
  phases: [
    { title: "Scope & Safety", detail: "Normalize input, cap audit width, and enforce read-only audit intent." },
    { title: "Stack Classification", detail: "Classify repository shape and select high-signal evidence files." },
    { title: "Parallel Boundary Audit", detail: "Audit privacy, provider routing, storage, telemetry, path authority, and secret exposure in parallel." },
    { title: "Adversarial Verification", detail: "Verify or refute candidate findings in fresh contexts." },
    { title: "Optional Structural Checks", detail: "Run narrow validation only when explicitly requested." },
    { title: "Synthesis", detail: "Return verdict, confirmed findings, evidence files, open risks, and next actions." }
  ]
};

const DEFAULT_SCOPE = "repo";
const DEFAULT_MAX_FILES_PER_SURFACE = 12;
const DEFAULT_MAX_TOTAL_FINDINGS = 30;
const HARD_MAX_FILES_PER_SURFACE = 30;
const HARD_MAX_TOTAL_FINDINGS = 80;

const RISK_SURFACES = [
  {
    id: "privacy-boundaries",
    title: "Privacy Boundaries",
    label: "audit privacy",
    focus:
      "PII handling, consent boundaries, local-vs-remote data movement, clipboard/screenshot/file access, prompt payload construction, redaction, and privacy docs-to-code drift."
  },
  {
    id: "provider-routing",
    title: "Provider Routing",
    label: "audit providers",
    focus:
      "model/provider selection, fallback behavior, endpoint routing, API key selection, environment-specific drift, provider capability checks, and accidental cross-provider leakage."
  },
  {
    id: "storage-recovery",
    title: "Storage & Recovery",
    label: "audit storage",
    focus:
      "local persistence, migrations, corruption handling, backup/restore, schema/version compatibility, atomic writes, and recovery paths after failed updates."
  },
  {
    id: "telemetry-release",
    title: "Telemetry & Release Evidence",
    label: "audit release",
    focus:
      "analytics payloads, opt-in/opt-out enforcement, crash logs, release gates, signing/update evidence, CI/CD artifacts, and telemetry leakage."
  },
  {
    id: "path-authority",
    title: "Raw Path Authority",
    label: "audit paths",
    focus:
      "raw file path handling, directory traversal, symlink boundaries, file picker trust, workspace-root constraints, sandbox escape, and path normalization."
  },
  {
    id: "secret-exposure",
    title: "Secret Exposure",
    label: "audit secrets",
    focus:
      "hardcoded credentials, committed tokens, secret logging, .env misuse, API keys in test fixtures, secrets in telemetry, and accidental disclosure. Never print secret values."
  }
];

const inventorySchema = {
  type: "object",
  additionalProperties: false,
  properties: {
    scope: { type: "string" },
    focusMode: { type: "string", enum: ["broad", "narrow"] },
    repoType: { type: "string" },
    primaryLanguages: { type: "array", items: { type: "string" } },
    packageManagers: { type: "array", items: { type: "string" } },
    frameworks: { type: "array", items: { type: "string" } },
    desktopRuntime: { type: "string" },
    configFiles: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          path: { type: "string" },
          purpose: { type: "string" }
        },
        required: ["path", "purpose"]
      }
    },
    candidateFilesBySurface: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          surface: { type: "string" },
          files: {
            type: "array",
            items: {
              type: "object",
              additionalProperties: false,
              properties: {
                path: { type: "string" },
                reason: { type: "string" },
                confidence: { type: "string", enum: ["low", "medium", "high"] }
              },
              required: ["path", "reason", "confidence"]
            }
          }
        },
        required: ["surface", "files"]
      }
    },
    likelyTestCommands: { type: "array", items: { type: "string" } },
    warnings: { type: "array", items: { type: "string" } }
  },
  required: [
    "scope",
    "focusMode",
    "repoType",
    "primaryLanguages",
    "packageManagers",
    "frameworks",
    "desktopRuntime",
    "configFiles",
    "candidateFilesBySurface",
    "likelyTestCommands",
    "warnings"
  ]
};

const auditSchema = {
  type: "object",
  additionalProperties: false,
  properties: {
    surface: { type: "string" },
    inspectedFiles: { type: "array", items: { type: "string" } },
    findings: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          title: { type: "string" },
          severity: { type: "string", enum: ["critical", "high", "medium", "low", "info"] },
          confidence: { type: "string", enum: ["low", "medium", "high"] },
          claim: { type: "string" },
          evidence: {
            type: "array",
            items: {
              type: "object",
              additionalProperties: false,
              properties: {
                path: { type: "string" },
                symbolOrArea: { type: "string" },
                lineHint: { type: "string" },
                observation: { type: "string" }
              },
              required: ["path", "symbolOrArea", "lineHint", "observation"]
            }
          },
          impact: { type: "string" },
          reproductionOrCheck: { type: "string" },
          recommendation: { type: "string" }
        },
        required: [
          "title",
          "severity",
          "confidence",
          "claim",
          "evidence",
          "impact",
          "reproductionOrCheck",
          "recommendation"
        ]
      }
    },
    openRisks: { type: "array", items: { type: "string" } },
    noFindingRationale: { type: "string" }
  },
  required: ["surface", "inspectedFiles", "findings", "openRisks", "noFindingRationale"]
};

const verificationSchema = {
  type: "object",
  additionalProperties: false,
  properties: {
    surface: { type: "string" },
    verdicts: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          findingId: { type: "string" },
          verdict: { type: "string", enum: ["keep", "refute", "needs-human"] },
          rationale: { type: "string" },
          evidenceQuality: { type: "string", enum: ["weak", "adequate", "strong"] },
          requiredFollowUp: { type: "string" }
        },
        required: ["findingId", "verdict", "rationale", "evidenceQuality", "requiredFollowUp"]
      }
    },
    verifierNotes: { type: "array", items: { type: "string" } }
  },
  required: ["surface", "verdicts", "verifierNotes"]
};

const structuralCheckSchema = {
  type: "object",
  additionalProperties: false,
  properties: {
    commandsConsidered: { type: "array", items: { type: "string" } },
    commandsRun: { type: "array", items: { type: "string" } },
    result: { type: "string", enum: ["passed", "failed", "skipped"] },
    outputSummary: { type: "string" },
    followUp: { type: "array", items: { type: "string" } }
  },
  required: ["commandsConsidered", "commandsRun", "result", "outputSummary", "followUp"]
};

const finalReportSchema = {
  type: "object",
  additionalProperties: false,
  properties: {
    auditVerdict: {
      type: "string",
      enum: ["pass", "pass-with-risks", "needs-fix", "blocked", "inconclusive"]
    },
    executiveSummary: { type: "string" },
    confirmedFindings: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          findingId: { type: "string" },
          surface: { type: "string" },
          title: { type: "string" },
          severity: { type: "string", enum: ["critical", "high", "medium", "low", "info"] },
          confidence: { type: "string", enum: ["low", "medium", "high"] },
          evidenceFiles: { type: "array", items: { type: "string" } },
          evidenceSummary: { type: "string" },
          impact: { type: "string" },
          recommendation: { type: "string" },
          verificationRationale: { type: "string" }
        },
        required: [
          "findingId",
          "surface",
          "title",
          "severity",
          "confidence",
          "evidenceFiles",
          "evidenceSummary",
          "impact",
          "recommendation",
          "verificationRationale"
        ]
      }
    },
    evidenceFiles: { type: "array", items: { type: "string" } },
    openRisks: { type: "array", items: { type: "string" } },
    recommendedNextActions: {
      type: "array",
      items: {
        type: "object",
        additionalProperties: false,
        properties: {
          priority: { type: "string", enum: ["p0", "p1", "p2", "p3"] },
          action: { type: "string" },
          ownerHint: { type: "string" },
          validation: { type: "string" }
        },
        required: ["priority", "action", "ownerHint", "validation"]
      }
    },
    structuralCheck: {
      type: "object",
      additionalProperties: false,
      properties: {
        result: { type: "string", enum: ["passed", "failed", "skipped"] },
        summary: { type: "string" }
      },
      required: ["result", "summary"]
    },
    limitations: { type: "array", items: { type: "string" } },
    workflowContract: {
      type: "object",
      additionalProperties: false,
      properties: {
        workflow: { type: "string" },
        version: { type: "number" },
        scope: { type: "string" },
        focusMode: { type: "string" },
        riskSurfaces: { type: "array", items: { type: "string" } },
        outputSections: { type: "array", items: { type: "string" } }
      },
      required: ["workflow", "version", "scope", "focusMode", "riskSurfaces", "outputSections"]
    }
  },
  required: [
    "auditVerdict",
    "executiveSummary",
    "confirmedFindings",
    "evidenceFiles",
    "openRisks",
    "recommendedNextActions",
    "structuralCheck",
    "limitations",
    "workflowContract"
  ]
};

function clampNumber(value, fallback, hardMax) {
  const n = Number(value);
  if (!Number.isFinite(n) || n <= 0) return fallback;
  return Math.min(Math.floor(n), hardMax);
}

function normalizeArgs(input) {
  let parsed = input;

  if (typeof parsed === "string") {
    const trimmed = parsed.trim();
    if (!trimmed) parsed = {};
    else {
      try {
        parsed = JSON.parse(trimmed);
      } catch {
        parsed = { scope: trimmed };
      }
    }
  }

  if (!parsed || typeof parsed !== "object") parsed = {};

  const scope = typeof parsed.scope === "string" && parsed.scope.trim()
    ? parsed.scope.trim()
    : DEFAULT_SCOPE;

  return {
    scope,
    focusMode: scope === DEFAULT_SCOPE ? "broad" : "narrow",
    maxFilesPerSurface: clampNumber(
      parsed.maxFilesPerSurface,
      DEFAULT_MAX_FILES_PER_SURFACE,
      HARD_MAX_FILES_PER_SURFACE
    ),
    maxTotalFindings: clampNumber(
      parsed.maxTotalFindings,
      DEFAULT_MAX_TOTAL_FINDINGS,
      HARD_MAX_TOTAL_FINDINGS
    ),
    runStructuralChecks: parsed.runStructuralChecks === true,
    dryRun: parsed.dryRun === true,
    riskSurfaces: Array.isArray(parsed.riskSurfaces) ? parsed.riskSurfaces : []
  };
}

function safeStringify(value) {
  return JSON.stringify(value, null, 2);
}

async function runAgent(mockKey, prompt, options, config) {
  if (config.dryRun) {
    log(`[DRY-RUN] ${options.label || mockKey}`);
    if (mockKey === "inventory") {
      return {
        scope: config.scope,
        focusMode: config.focusMode,
        repoType: "dry-run-repo",
        primaryLanguages: [],
        packageManagers: [],
        frameworks: [],
        desktopRuntime: "unknown",
        configFiles: [],
        candidateFilesBySurface: RISK_SURFACES.map((surface) => ({
          surface: surface.id,
          files: []
        })),
        likelyTestCommands: [],
        warnings: ["Dry run enabled: no repository files were inspected."]
      };
    }
    if (mockKey.indexOf("audit:") === 0) {
      return {
        surface: mockKey.slice("audit:".length),
        inspectedFiles: [],
        findings: [],
        openRisks: ["Dry run enabled."],
        noFindingRationale: "No live audit was executed."
      };
    }
    if (mockKey.indexOf("verify:") === 0) {
      return {
        surface: mockKey.slice("verify:".length),
        verdicts: [],
        verifierNotes: ["Dry run enabled."]
      };
    }
    if (mockKey === "structural-check") {
      return {
        commandsConsidered: [],
        commandsRun: [],
        result: "skipped",
        outputSummary: "Dry run enabled.",
        followUp: []
      };
    }
    return {
      auditVerdict: "inconclusive",
      executiveSummary: "Dry run completed successfully. No live repository inspection was performed.",
      confirmedFindings: [],
      evidenceFiles: [],
      openRisks: ["Dry run enabled."],
      recommendedNextActions: [
        {
          priority: "p2",
          action: "Run the workflow without dryRun to perform the real audit.",
          ownerHint: "workflow runner",
          validation: "Workflow returns live evidence-backed findings."
        }
      ],
      structuralCheck: { result: "skipped", summary: "Dry run enabled." },
      limitations: ["Dry run enabled."],
      workflowContract: {
        workflow: "repo-audit",
        version: 2,
        scope: config.scope,
        focusMode: config.focusMode,
        riskSurfaces: selectedSurfaces(config).map((s) => s.id),
        outputSections: [
          "audit-verdict",
          "confirmed-findings",
          "evidence-files",
          "open-risks",
          "recommended-next-action"
        ]
      }
    };
  }

  return await agent(prompt, options);
}

function selectedSurfaces(config) {
  if (!config.riskSurfaces.length) return RISK_SURFACES;
  const wanted = {};
  for (const id of config.riskSurfaces) wanted[String(id)] = true;
  return RISK_SURFACES.filter((surface) => wanted[surface.id]);
}

function candidateFilesForSurface(inventory, surfaceId) {
  const groups = Array.isArray(inventory.candidateFilesBySurface)
    ? inventory.candidateFilesBySurface
    : [];

  const group = groups.find((entry) => entry.surface === surfaceId);
  if (!group || !Array.isArray(group.files)) return [];

  return group.files
    .map((file) => file && file.path)
    .filter(Boolean);
}

function annotateFindings(auditResults) {
  const findings = [];

  for (const result of auditResults) {
    if (!result || !Array.isArray(result.findings)) continue;

    for (let i = 0; i < result.findings.length; i += 1) {
      const finding = result.findings[i];
      findings.push({
        findingId: `${result.surface || "surface"}-${i + 1}`,
        surface: result.surface || "unknown",
        ...finding
      });
    }
  }

  return findings;
}

function collectOpenRisks(auditResults) {
  const risks = [];
  for (const result of auditResults) {
    if (!result || !Array.isArray(result.openRisks)) continue;
    for (const risk of result.openRisks) {
      risks.push(`${result.surface}: ${risk}`);
    }
  }
  return risks;
}

function filterFindings(annotatedFindings, verificationResults) {
  const verdictById = {};

  for (const verification of verificationResults) {
    if (!verification || !Array.isArray(verification.verdicts)) continue;
    for (const verdict of verification.verdicts) {
      verdictById[verdict.findingId] = verdict;
    }
  }

  const confirmed = [];
  const disputed = [];

  for (const finding of annotatedFindings) {
    const verdict = verdictById[finding.findingId];

    if (verdict && verdict.verdict === "keep") {
      confirmed.push({
        ...finding,
        verificationRationale: verdict.rationale,
        evidenceQuality: verdict.evidenceQuality
      });
    } else {
      disputed.push({
        ...finding,
        verificationRationale: verdict ? verdict.rationale : "No verifier verdict returned.",
        verifierVerdict: verdict ? verdict.verdict : "missing"
      });
    }
  }

  return { confirmed, disputed };
}

async function classifyStack(config, surfaces) {
  phase("Stack Classification");
  log(`Classifying scope '${config.scope}' with a per-surface cap of ${config.maxFilesPerSurface}.`);

  const prompt =
    `You are the discovery agent for a read-only repo-audit workflow.\n\n` +
    `Scope: ${config.scope}\n` +
    `Focus mode: ${config.focusMode}\n` +
    `Risk surfaces:\n${surfaces.map((s) => `- ${s.id}: ${s.title}`).join("\n")}\n\n` +
    `Instructions:\n` +
    `1. Classify the stack and repository shape using targeted file discovery.\n` +
    `2. Select at most ${config.maxFilesPerSurface} high-signal files per risk surface.\n` +
    `3. Prefer source files, config, release manifests, tests proving boundary contracts, and docs only when they establish intended behavior.\n` +
    `4. Stay inside the requested scope. If scope is broad, sample highest-risk boundary files rather than listing the entire repository.\n` +
    `5. Do not edit files. Do not print secret values. For suspected secrets, report only paths, key names, and risk descriptions.\n` +
    `6. Return structured data only.`;

  return await runAgent(
    "inventory",
    prompt,
    { label: "classify stack", schema: inventorySchema },
    config
  );
}

async function auditSurface(config, inventory, surface) {
  const candidateFiles = candidateFilesForSurface(inventory, surface.id);
  const fileList = candidateFiles.length
    ? candidateFiles.join("\n")
    : "No candidate files were identified; use narrowly targeted search inside scope.";

  const prompt =
    `You are the ${surface.title} specialist in a read-only repo-audit workflow.\n\n` +
    `Scope: ${config.scope}\n` +
    `Surface: ${surface.id}\n` +
    `Focus: ${surface.focus}\n\n` +
    `Candidate files:\n${fileList}\n\n` +
    `Repository inventory:\n${safeStringify(inventory)}\n\n` +
    `Audit rules:\n` +
    `- Inspect only candidate files and directly adjacent files required to verify claims.\n` +
    `- Do not edit files. Avoid broad test execution. Do not reveal secret values.\n` +
    `- Prefer code evidence over documentation claims.\n` +
    `- Documentation-only drift can be a finding only when implementation contradicts a stated contract.\n` +
    `- Every finding must include concrete file evidence with path, symbol/area, line hints when available, and observation.\n` +
    `- Do not report plausible risks as confirmed findings. Put uncertain items in openRisks.\n` +
    `- Sort findings by severity and confidence. Return structured data only.`;

  return await runAgent(
    `audit:${surface.id}`,
    prompt,
    { label: surface.label, schema: auditSchema },
    config
  );
}

async function verifySurface(config, surface, findingsForSurface) {
  const prompt =
    `You are an adversarial verifier for the repo-audit workflow.\n\n` +
    `Surface: ${surface.id}\n` +
    `Scope: ${config.scope}\n\n` +
    `Findings to verify:\n${safeStringify(findingsForSurface)}\n\n` +
    `Verification rubric:\n` +
    `- Try to refute each finding using repository files and provided evidence.\n` +
    `- Keep a finding only when code evidence supports the claim and impact is not overstated.\n` +
    `- Use "refute" when the claim is contradicted or unsupported.\n` +
    `- Use "needs-human" when external context, credentials, production config, or runtime behavior is required.\n` +
    `- Never add new findings. Never reveal secret values. Return structured data only.`;

  return await runAgent(
    `verify:${surface.id}`,
    prompt,
    { label: `verify ${surface.id}`, schema: verificationSchema },
    config
  );
}

async function runStructuralChecks(config, inventory, confirmedFindings) {
  if (!config.runStructuralChecks) {
    return {
      commandsConsidered: inventory.likelyTestCommands || [],
      commandsRun: [],
      result: "skipped",
      outputSummary: "Structural checks skipped because runStructuralChecks was not true.",
      followUp: [
        "Run with { runStructuralChecks: true } after reviewing command allowlists and workflow plan."
      ]
    };
  }

  phase("Optional Structural Checks");
  log("Requesting narrow read-only structural verification for confirmed findings.");

  const prompt =
    `Run the narrowest safe verification available for these confirmed audit findings.\n\n` +
    `Rules:\n` +
    `- Do not edit files.\n` +
    `- Prefer targeted tests, type checks, lint checks, or static validation listed in package scripts.\n` +
    `- Do not run destructive commands, migrations, network calls, release publishing, or commands requiring credentials.\n` +
    `- If no safe narrow command exists, skip and explain why.\n\n` +
    `Likely commands from inventory:\n${safeStringify(inventory.likelyTestCommands || [])}\n\n` +
    `Confirmed findings:\n${safeStringify(confirmedFindings)}\n\n` +
    `Return structured data only.`;

  return await runAgent(
    "structural-check",
    prompt,
    { label: "structural check", schema: structuralCheckSchema },
    config
  );
}

async function synthesizeReport(config, surfaces, inventory, auditResults, verificationResults, filtered, structuralCheck, wasTruncated) {
  phase("Synthesis");
  log("Synthesizing verified audit findings into final report.");

  const prompt =
    `Synthesize the repo-audit workflow output.\n\n` +
    `Required output sections to preserve: audit-verdict, confirmed-findings, evidence-files, open-risks, recommended-next-action.\n` +
    `Scope: ${config.scope}\n` +
    `Focus mode: ${config.focusMode}\n` +
    `Risk surfaces: ${surfaces.map((s) => s.id).join(", ")}\n\n` +
    `Inventory:\n${safeStringify(inventory)}\n\n` +
    `Raw audit results:\n${safeStringify(auditResults)}\n\n` +
    `Verification results:\n${safeStringify(verificationResults)}\n\n` +
    `Confirmed findings after verifier gate:\n${safeStringify(filtered.confirmed)}\n\n` +
    `Disputed or human-review findings:\n${safeStringify(filtered.disputed)}\n\n` +
    `Open risks from auditors:\n${safeStringify(collectOpenRisks(auditResults))}\n\n` +
    `Structural check result:\n${safeStringify(structuralCheck)}\n\n` +
    `Finding cap reached: ${wasTruncated ? "yes" : "no"}\n\n` +
    `Report rules:\n` +
    `- Include only verifier-kept findings in confirmedFindings.\n` +
    `- Preserve evidence paths exactly.\n` +
    `- Move refuted or uncertain items into openRisks or limitations, not confirmedFindings.\n` +
    `- Verdict guidance: blocked for critical/high confirmed release blockers; needs-fix for actionable medium+ issues; pass-with-risks for low/uncertain risks; pass only when no material findings and low open risk; inconclusive when evidence coverage was insufficient.\n` +
    `- Return structured data only.`;

  return await runAgent(
    "final-report",
    prompt,
    { label: "synthesize report", schema: finalReportSchema },
    config
  );
}

async function runRepoAudit() {
  phase("Scope & Safety");

  const config = normalizeArgs(typeof args === "undefined" ? undefined : args);
  const surfaces = selectedSurfaces(config);

  if (!surfaces.length) {
    return {
      auditVerdict: "blocked",
      executiveSummary: "No recognized risk surfaces were selected.",
      confirmedFindings: [],
      evidenceFiles: [],
      openRisks: ["Requested riskSurfaces did not match supported surfaces."],
      recommendedNextActions: [
        {
          priority: "p1",
          action:
            "Run with supported surfaces: privacy-boundaries, provider-routing, storage-recovery, telemetry-release, path-authority, secret-exposure.",
          ownerHint: "workflow runner",
          validation: "Workflow reaches Stack Classification."
        }
      ],
      structuralCheck: { result: "skipped", summary: "No audit was run." },
      limitations: ["Invalid input."],
      workflowContract: {
        workflow: "repo-audit",
        version: 2,
        scope: config.scope,
        focusMode: config.focusMode,
        riskSurfaces: [],
        outputSections: [
          "audit-verdict",
          "confirmed-findings",
          "evidence-files",
          "open-risks",
          "recommended-next-action"
        ]
      }
    };
  }

  log(
    `Starting repo-audit v2 for '${config.scope}' across ${surfaces.length} risk surfaces. ` +
    `Read-only intent; agents must not edit files.`
  );

  const inventory = await classifyStack(config, surfaces);

  phase("Parallel Boundary Audit");
  log(`Running ${surfaces.length} scoped audit agents.`);

  const auditResultsRaw = await parallel(
    surfaces.map((surface) => () => auditSurface(config, inventory, surface))
  );

  const auditResults = auditResultsRaw.filter(Boolean);
  const allFindings = annotateFindings(auditResults);
  const wasTruncated = allFindings.length > config.maxTotalFindings;
  const cappedFindings = allFindings.slice(0, config.maxTotalFindings);

  if (wasTruncated) {
    log(
      `Candidate findings exceeded cap of ${config.maxTotalFindings}; ` +
      `rerun with narrower scope for exhaustive coverage.`
    );
  }

  phase("Adversarial Verification");
  log(`Verifying ${cappedFindings.length} candidate findings.`);

  const verificationTasks = [];
  for (const surface of surfaces) {
    const scoped = cappedFindings.filter((finding) => finding.surface === surface.id);
    if (scoped.length) {
      verificationTasks.push(() => verifySurface(config, surface, scoped));
    }
  }

  const verificationResultsRaw = verificationTasks.length
    ? await parallel(verificationTasks)
    : [];

  const verificationResults = verificationResultsRaw.filter(Boolean);
  const filtered = filterFindings(cappedFindings, verificationResults);
  const structuralCheck = await runStructuralChecks(config, inventory, filtered.confirmed);

  const finalReport = await synthesizeReport(
    config,
    surfaces,
    inventory,
    auditResults,
    verificationResults,
    filtered,
    structuralCheck,
    wasTruncated
  );

  if (wasTruncated && Array.isArray(finalReport.limitations)) {
    finalReport.limitations.push(
      `Candidate findings were capped at ${config.maxTotalFindings}; rerun with a narrower scope for exhaustive coverage.`
    );
  }

  return finalReport;
}

return await runRepoAudit();