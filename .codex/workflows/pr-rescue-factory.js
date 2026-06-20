export const meta = {
  name: "pr_rescue_factory",
  description: "Triages stalled or specified PRs for merge conflicts, CI failures, review blockers, stale branches, and failed checks. Uses bounded parallel diagnostics, evidence auditing, optional isolated patch attempts, and final safe-merge queue synthesis.",
  whenToUse: "Run when open pull requests are stalled, failing CI, conflicted, or awaiting release triage. Pass args.baseBranch, args.prs='open' or args.prs=[1,2,3]. Default fixMode='suggest' is read-only. fixMode='apply' may edit only inside isolated worktrees and never pushes, merges, closes, or approves PRs.",
  phases: [
    { title: "Preflight & Scope Validation", detail: "Validate args, gh auth, repository access, target base branch, worktree readiness, and token budget envelope" },
    { title: "PR Inventory & Classification", detail: "Fetch normalized PR metadata and choose the bounded triage scope" },
    { title: "Parallel Diagnostic & Fix Proposal", detail: "Fan out one bounded diagnostic worker per PR with strict evidence and intent-preservation schemas" },
    { title: "Evidence Audit & Merge Gate", detail: "Adversarially inspect diagnostic evidence for unsupported claims, unsafe merge suggestions, and missing verification" },
    { title: "Risk Synthesis & Safe Merge Queue", detail: "Rank ready PRs, block unsafe PRs, and produce a human-reviewable remediation playbook" }
  ],
  productionConfig: {
    budgetUSD: 8.0,
    validationRequirements: [
      "gh-auth",
      "repo-access",
      "base-branch-exists",
      "intent-evidence-present",
      "verification-command-present",
      "do-not-merge-contract"
    ],
    stateRecoveryLedger: "./.claude/pr_rescue_passport.json",
    compensationProtocol: "none for suggest mode; apply mode is isolated to worktrees and must not push, merge, close, approve, or commit without human review",
    isolationConfig: {
      useDatabaseIsolation: false,
      environmentFiles: []
    }
  }
};

/**
 * PR Rescue Factory — hardened dynamic workflow
 *
 * Architecture:
 * - Classify-and-Act: normalize PR inventory and blocker classes.
 * - Fan-Out-and-Synthesize: diagnose each PR in bounded parallel batches.
 * - Adversarial Verification: audit evidence before merge queue synthesis.
 * - Generate-and-Filter: generate minimal fix proposals, then filter by safety.
 *
 * Hardening:
 * - Deterministic JS only: no Date.now(), Math.random(), imports, fs, or network calls in orchestrator.
 * - gh/repo/base preflight before expensive fan-out.
 * - Prompt/schema alignment on every agent call.
 * - Concurrency clamped to workflow cap.
 * - Token preflight and mid-run budget checks.
 * - Failure-rate early exit.
 * - Dry-run mode that validates scope and budget without running PR triage workers.
 * - apply mode is isolated and explicitly forbidden from pushing/merging/closing/approving.
 */

const WORKER_MODEL = "claude-sonnet-4-6";
const REVIEW_MODEL = "claude-opus-4-8";

const HARD_CONCURRENCY_CAP = 16;
const DEFAULT_MAX_CONCURRENT = 6;
const DEFAULT_MAX_PRS = 20;
const DEFAULT_TOKEN_SAFETY_CEILING = 2500000;
const DEFAULT_WARNING_TOKEN_THRESHOLD = 1800000;
const EST_INVENTORY_TOKENS = 12000;
const EST_SUGGEST_TOKENS_PER_PR = 30000;
const EST_APPLY_TOKENS_PER_PR = 52000;
const EST_AUDIT_TOKENS = 18000;
const EST_SYNTHESIS_TOKENS = 25000;

function runtimeArgs() {
  return typeof args !== "undefined" && args ? args : {};
}

function budgetTotal() {
  if (typeof budget === "undefined" || !budget) return 0;
  return typeof budget.total === "number" ? budget.total : 0;
}

function isArray(value) {
  return Object.prototype.toString.call(value) === "[object Array]";
}

function toInteger(value, fallback) {
  const n = Number(value);
  if (!Number.isFinite(n)) return fallback;
  return Math.floor(n);
}

function clampInteger(value, min, max, fallback) {
  const n = toInteger(value, fallback);
  if (n < min) return min;
  if (n > max) return max;
  return n;
}

function toBoolean(value, fallback) {
  if (value === true || value === false) return value;
  if (value === "true") return true;
  if (value === "false") return false;
  return fallback;
}

function isSafeBranchName(value) {
  if (typeof value !== "string") return false;
  if (value.length < 1 || value.length > 160) return false;
  return /^[A-Za-z0-9._\/-]+$/.test(value);
}

function normalizeFixMode(value) {
  if (value === "apply") return "apply";
  return "suggest";
}

function sanitizeText(value, fallback) {
  if (value === null || value === undefined) return fallback || "";
  return String(value).replace(/`/g, "'").slice(0, 2000);
}

function normalizePrInput(value) {
  if (isArray(value)) {
    return {
      mode: "explicit",
      numbers: uniquePositiveIntegers(value)
    };
  }

  if (typeof value === "number") {
    return {
      mode: "explicit",
      numbers: uniquePositiveIntegers([value])
    };
  }

  if (typeof value === "string") {
    const trimmed = value.trim();
    if (trimmed === "" || trimmed === "open") {
      return { mode: "open", numbers: [] };
    }

    const parts = trimmed.split(",");
    const parsed = [];
    for (let i = 0; i < parts.length; i++) {
      parsed.push(parts[i].trim());
    }

    const numbers = uniquePositiveIntegers(parsed);
    if (numbers.length > 0) {
      return { mode: "explicit", numbers };
    }
  }

  return { mode: "open", numbers: [] };
}

function uniquePositiveIntegers(values) {
  const seen = {};
  const result = [];

  for (let i = 0; i < values.length; i++) {
    const n = toInteger(values[i], -1);
    if (n > 0 && !seen[String(n)]) {
      seen[String(n)] = true;
      result.push(n);
    }
  }

  return result;
}

function inventoryByPrNumber(prs) {
  const map = {};
  for (let i = 0; i < prs.length; i++) {
    if (prs[i] && typeof prs[i].number === "number") {
      map[String(prs[i].number)] = prs[i];
    }
  }
  return map;
}

function buildAgentOptions(label, model, schema, useWorktree) {
  const opts = {
    label,
    model,
    schema
  };

  if (useWorktree) {
    opts.isolation = "worktree";
  }

  return opts;
}

function estimatedTokenCost(targetCount, fixMode, auditEvidence) {
  const perPr = fixMode === "apply" ? EST_APPLY_TOKENS_PER_PR : EST_SUGGEST_TOKENS_PER_PR;
  return EST_INVENTORY_TOKENS +
    (targetCount * perPr) +
    (auditEvidence ? EST_AUDIT_TOKENS : 0) +
    EST_SYNTHESIS_TOKENS;
}

function currentBudgetWouldExceed(projectedTokens, ceiling) {
  return budgetTotal() + projectedTokens > ceiling;
}

const preflightSchema = {
  type: "object",
  properties: {
    ok: { type: "boolean" },
    repo: {
      type: "object",
      properties: {
        owner: { type: "string" },
        name: { type: "string" },
        defaultBranch: { type: "string" },
        remoteUrlRedacted: { type: "string" }
      },
      required: ["owner", "name"]
    },
    checks: {
      type: "object",
      properties: {
        ghAuthenticated: { type: "boolean" },
        insideGitRepo: { type: "boolean" },
        repoAccessible: { type: "boolean" },
        baseBranchExists: { type: "boolean" },
        workingTreeReadable: { type: "boolean" },
        worktreeSupported: { type: "boolean" }
      },
      required: [
        "ghAuthenticated",
        "insideGitRepo",
        "repoAccessible",
        "baseBranchExists",
        "workingTreeReadable",
        "worktreeSupported"
      ]
    },
    failures: {
      type: "array",
      items: { type: "string" }
    },
    warnings: {
      type: "array",
      items: { type: "string" }
    },
    evidence: {
      type: "array",
      items: {
        type: "object",
        properties: {
          command: { type: "string" },
          summary: { type: "string" }
        },
        required: ["command", "summary"]
      }
    }
  },
  required: ["ok", "checks", "failures", "warnings"]
};

const inventorySchema = {
  type: "object",
  properties: {
    prs: {
      type: "array",
      items: {
        type: "object",
        properties: {
          number: { type: "number" },
          title: { type: "string" },
          url: { type: "string" },
          baseRefName: { type: "string" },
          headRefName: { type: "string" },
          mergeable: { type: "string" },
          reviewDecision: { type: "string" },
          checksStatus: { type: "string" },
          failingChecks: {
            type: "array",
            items: { type: "string" }
          },
          filesChanged: { type: "number" },
          author: { type: "string" },
          isDraft: { type: "boolean" },
          updatedAt: { type: "string" },
          bodyExcerpt: { type: "string" },
          labels: {
            type: "array",
            items: { type: "string" }
          }
        },
        required: [
          "number",
          "title",
          "baseRefName",
          "headRefName",
          "mergeable",
          "checksStatus",
          "filesChanged",
          "author",
          "isDraft"
        ]
      }
    },
    truncated: { type: "boolean" },
    discoveryNotes: { type: "string" },
    warnings: {
      type: "array",
      items: { type: "string" }
    }
  },
  required: ["prs", "truncated", "warnings"]
};

const triageSchema = {
  type: "object",
  properties: {
    prNumber: { type: "number" },
    title: { type: "string" },
    failureClass: {
      type: "string",
      enum: [
        "merge_conflict",
        "ci_failure",
        "stale_branch",
        "review_required",
        "draft_or_policy_blocked",
        "clean_ready",
        "other"
      ]
    },
    currentState: {
      type: "string",
      enum: ["ready", "blocked", "needs_fix", "needs_review", "unknown"]
    },
    intentSummary: { type: "string", minLength: 20 },
    rootCause: { type: "string", minLength: 10 },
    rootCauseConfidence: {
      type: "string",
      enum: ["high", "medium", "low"]
    },
    affectedFiles: {
      type: "array",
      items: { type: "string" }
    },
    failingChecks: {
      type: "array",
      items: {
        type: "object",
        properties: {
          name: { type: "string" },
          conclusion: { type: "string" },
          evidence: { type: "string" }
        },
        required: ["name", "conclusion", "evidence"]
      }
    },
    proposedFix: {
      type: "object",
      properties: {
        summary: { type: "string" },
        files: {
          type: "array",
          items: { type: "string" }
        },
        rationale: { type: "string" },
        estimatedComplexity: {
          type: "string",
          enum: ["none", "low", "medium", "high"]
        }
      },
      required: ["summary", "files", "rationale", "estimatedComplexity"]
    },
    verification: {
      type: "object",
      properties: {
        commands: {
          type: "array",
          items: { type: "string" }
        },
        evidenceRequired: {
          type: "array",
          items: { type: "string" }
        },
        currentEvidence: {
          type: "array",
          items: { type: "string" }
        },
        status: {
          type: "string",
          enum: ["not_run", "failing", "passing", "partial", "not_applicable"]
        }
      },
      required: ["commands", "evidenceRequired", "currentEvidence", "status"]
    },
    applyResult: {
      type: "object",
      properties: {
        attempted: { type: "boolean" },
        applied: { type: "boolean" },
        changedFiles: {
          type: "array",
          items: { type: "string" }
        },
        diffSummary: { type: "string" },
        verificationStatus: {
          type: "string",
          enum: ["not_attempted", "not_run", "failing", "passing", "partial"]
        },
        notes: { type: "string" }
      },
      required: ["attempted", "applied", "changedFiles", "diffSummary", "verificationStatus", "notes"]
    },
    doNotMergeIf: {
      type: "array",
      items: { type: "string" }
    },
    mergeRisk: {
      type: "string",
      enum: ["low", "medium", "high"]
    },
    readySignal: {
      type: "string",
      enum: ["merge_now", "fix_then_merge", "needs_human_review", "blocked"]
    },
    evidence: {
      type: "array",
      items: {
        type: "object",
        properties: {
          source: { type: "string" },
          excerpt: { type: "string" },
          supports: { type: "string" }
        },
        required: ["source", "excerpt", "supports"]
      }
    },
    humanReviewNotes: { type: "string" }
  },
  required: [
    "prNumber",
    "title",
    "failureClass",
    "currentState",
    "intentSummary",
    "rootCause",
    "rootCauseConfidence",
    "affectedFiles",
    "proposedFix",
    "verification",
    "applyResult",
    "doNotMergeIf",
    "mergeRisk",
    "readySignal",
    "evidence",
    "humanReviewNotes"
  ]
};

const evidenceAuditSchema = {
  type: "object",
  properties: {
    overallEvidenceQuality: {
      type: "string",
      enum: ["strong", "mixed", "weak"]
    },
    approvedPrNumbers: {
      type: "array",
      items: { type: "number" }
    },
    needsRecheck: {
      type: "array",
      items: {
        type: "object",
        properties: {
          prNumber: { type: "number" },
          reason: { type: "string" },
          missingEvidence: {
            type: "array",
            items: { type: "string" }
          }
        },
        required: ["prNumber", "reason", "missingEvidence"]
      }
    },
    unsafeMergeSuggestions: {
      type: "array",
      items: {
        type: "object",
        properties: {
          prNumber: { type: "number" },
          reason: { type: "string" }
        },
        required: ["prNumber", "reason"]
      }
    },
    suspectedSymptomPatches: {
      type: "array",
      items: {
        type: "object",
        properties: {
          prNumber: { type: "number" },
          reason: { type: "string" }
        },
        required: ["prNumber", "reason"]
      }
    },
    globalWarnings: {
      type: "array",
      items: { type: "string" }
    }
  },
  required: [
    "overallEvidenceQuality",
    "approvedPrNumbers",
    "needsRecheck",
    "unsafeMergeSuggestions",
    "suspectedSymptomPatches",
    "globalWarnings"
  ]
};

const synthesisSchema = {
  type: "object",
  properties: {
    safeMergeQueue: {
      type: "array",
      items: {
        type: "object",
        properties: {
          order: { type: "number" },
          prNumber: { type: "number" },
          title: { type: "string" },
          safetyScore: { type: "number" },
          status: {
            type: "string",
            enum: ["ready_to_merge", "verify_then_merge", "fix_required", "human_review_required", "blocked"]
          },
          rationale: { type: "string" },
          suggestedNextAction: { type: "string" },
          verificationCommands: {
            type: "array",
            items: { type: "string" }
          },
          riskFactors: {
            type: "array",
            items: { type: "string" }
          }
        },
        required: [
          "order",
          "prNumber",
          "title",
          "safetyScore",
          "status",
          "rationale",
          "suggestedNextAction",
          "verificationCommands",
          "riskFactors"
        ]
      }
    },
    blockedPRs: {
      type: "array",
      items: {
        type: "object",
        properties: {
          prNumber: { type: "number" },
          title: { type: "string" },
          blockerType: { type: "string" },
          blockingConditions: {
            type: "array",
            items: { type: "string" }
          },
          riskLevel: { type: "string" },
          remediation: { type: "string" }
        },
        required: ["prNumber", "title", "blockerType", "blockingConditions", "riskLevel", "remediation"]
      }
    },
    summaryStats: {
      type: "object",
      properties: {
        totalInScope: { type: "number" },
        totalAnalyzed: { type: "number" },
        readyToMerge: { type: "number" },
        verifyThenMerge: { type: "number" },
        needsWork: { type: "number" },
        blocked: { type: "number" },
        failedDiagnostics: { type: "number" },
        estimatedFixComplexity: {
          type: "string",
          enum: ["low", "medium", "high"]
        }
      },
      required: [
        "totalInScope",
        "totalAnalyzed",
        "readyToMerge",
        "verifyThenMerge",
        "needsWork",
        "blocked",
        "failedDiagnostics",
        "estimatedFixComplexity"
      ]
    },
    systemicFindings: {
      type: "array",
      items: { type: "string" }
    },
    remediationPlaybook: {
      type: "array",
      items: {
        type: "object",
        properties: {
          priority: { type: "number" },
          action: { type: "string" },
          ownerHint: { type: "string" },
          commands: {
            type: "array",
            items: { type: "string" }
          }
        },
        required: ["priority", "action", "ownerHint", "commands"]
      }
    },
    doNotMergeConditions: {
      type: "array",
      items: { type: "string" }
    },
    markdownReport: { type: "string" }
  },
  required: [
    "safeMergeQueue",
    "blockedPRs",
    "summaryStats",
    "systemicFindings",
    "remediationPlaybook",
    "doNotMergeConditions",
    "markdownReport"
  ]
};

async function runPreflight(config) {
  const prompt = `Validate that this repository is ready for PR rescue triage.

CONTEXT:
- target base branch: ${config.targetBase}
- fix mode: ${config.fixMode}
- apply mode requires git worktree support, but must not mutate anything during preflight.

RUN ONLY READ-ONLY COMMANDS:
1. gh auth status
2. git rev-parse --is-inside-work-tree
3. gh repo view --json owner,name,defaultBranchRef,url
4. git status --short
5. git branch --list ${config.targetBase}
6. gh pr list --base ${config.targetBase} --state open --limit 1 --json number

REDACTION:
- Do not expose tokens, credentials, or private environment variables.
- Redact remote URLs if they include credentials.

Return exactly the schema object. ok=false if gh auth, repo access, git repo, or base branch checks fail.`;

  return await agent(prompt, buildAgentOptions(
    "preflight",
    WORKER_MODEL,
    preflightSchema,
    false
  ));
}

async function runInventory(config) {
  let prompt = "";

  if (config.prScope.mode === "explicit") {
    prompt = `Fetch normalized metadata for these exact PR numbers only: ${config.prScope.numbers.join(", ")}.

Use gh CLI read-only commands. Prefer:
- gh pr view <number> --json number,title,url,baseRefName,headRefName,mergeable,reviewDecision,statusCheckRollup,files,author,body,isDraft,updatedAt,labels

For each PR:
- Summarize statusCheckRollup as checksStatus.
- Include failing check names in failingChecks.
- Set filesChanged to the count of changed files.
- Set bodyExcerpt to the first useful 500 characters, omitting secrets.
- Include labels as label names only.

Return exactly:
{
  "prs": [...],
  "truncated": false,
  "discoveryNotes": "...",
  "warnings": [...]
}`;
  } else {
    prompt = `List open PRs targeting base branch "${config.targetBase}", limited to ${config.maxPRs} PRs.

Use gh CLI read-only commands. Prefer:
- gh pr list --state open --base ${config.targetBase} --limit ${config.maxPRs} --json number,title,url,baseRefName,headRefName,mergeable,reviewDecision,statusCheckRollup,files,author,body,isDraft,updatedAt,labels

For each PR:
- Summarize statusCheckRollup as checksStatus.
- Include failing check names in failingChecks.
- Set filesChanged to the count of changed files.
- Set bodyExcerpt to the first useful 500 characters, omitting secrets.
- Include labels as label names only.
- Prioritize conflicted, failing, stale, draft, and review-blocked PRs if there are more than ${config.maxPRs} open PRs.

Return exactly:
{
  "prs": [...],
  "truncated": boolean,
  "discoveryNotes": "...",
  "warnings": [...]
}`;
  }

  return await agent(prompt, buildAgentOptions(
    "inventory-prs",
    WORKER_MODEL,
    inventorySchema,
    false
  ));
}

function makeTriagePrompt(pr, config) {
  const prTitle = sanitizeText(pr.title, "(untitled PR)");
  const prNumber = Number(pr.number);
  const modeInstruction = config.fixMode === "apply"
    ? `FIX MODE: apply.
You may attempt the smallest possible patch only inside your isolated worktree. Do not push, merge, close, approve, request changes, force-push, or commit. After any edit, run the narrowest useful verification command and report the diff summary.`
    : `FIX MODE: suggest.
Read-only analysis only. Do not edit files, create commits, push, merge, close, approve, request changes, or force-push.`;

  return `You are a senior staff engineer rescuing PR #${prNumber}: "${prTitle}" targeting ${config.targetBase}.

${modeInstruction}

KNOWN INVENTORY:
${JSON.stringify(pr, null, 2)}

MISSION:
Diagnose the primary blocker, preserve the PR author's original intent, propose the smallest safe fix, and define verifiable exit criteria. Be evidence-first and avoid generic advice.

MANDATORY READ-ONLY DIAGNOSTIC STEPS:
1. Inspect PR metadata/body/review state with gh pr view.
2. Inspect changed file list and targeted diff. Avoid dumping huge diffs; read only relevant sections.
3. Inspect check status. For failing checks, fetch the most relevant failed log excerpt.
4. Inspect mergeability/conflict evidence if mergeable indicates conflict or unknown.
5. Inspect recent commits enough to infer author intent.
6. Classify exactly one primary failureClass.
7. Separate root cause from symptom. Do not propose a symptom patch if the evidence points to a deeper cause.
8. Provide explicit doNotMergeIf conditions. Use [] only when there are no real unresolved conditions.
9. Provide verification commands that a human or later workflow can run.
10. Redact secrets from all excerpts.

CLASSIFICATION RULES:
- merge_conflict: merge conflicts are the primary blocker.
- ci_failure: failed checks/tests/build/lint are the primary blocker.
- stale_branch: branch is stale or mergeability is unknown primarily because it needs update with base.
- review_required: code is technically ready but review/approval is the primary blocker.
- draft_or_policy_blocked: draft state, policy label, missing linked issue, compliance gate, or release freeze blocks it.
- clean_ready: checks pass, mergeable, not draft, no review/policy blocker visible.
- other: only when none of the above fits.

APPLY MODE SAFETY:
If apply mode is active, apply only a minimal patch in the isolated worktree. Never touch unrelated files. If the fix requires broad architectural change, set applyResult.attempted=false and explain why.

Return only the JSON object matching the schema.`;
}

async function triageOnePr(pr, config) {
  const useWorktree = config.fixMode === "apply";
  const result = await agent(
    makeTriagePrompt(pr, config),
    buildAgentOptions(
      `triage-pr-${pr.number}`,
      WORKER_MODEL,
      triageSchema,
      useWorktree
    )
  );

  return result;
}

async function runTriageBatches(targets, config) {
  const tasks = targets.map((pr) => {
    return async () => {
      try {
        return await triageOnePr(pr, config);
      } catch (err) {
        log(`Task error on PR #${pr.number}: ${err && err.message ? err.message : String(err)}`);
        return null;
      }
    };
  });

  const triageResults = [];
  const failedPrs = [];
  let consecutiveFailedBatches = 0;

  for (let i = 0; i < tasks.length; i += config.maxConcurrent) {
    const batch = tasks.slice(i, i + config.maxConcurrent);
    const batchTargets = targets.slice(i, i + config.maxConcurrent);

    log(`Running PR diagnostic batch ${Math.floor(i / config.maxConcurrent) + 1}: ${batchTargets.map((p) => "#" + p.number).join(", ")}`);

    const batchResults = await parallel(batch);
    let batchFailures = 0;

    for (let j = 0; j < batchResults.length; j++) {
      const item = batchResults[j];
      if (item) {
        triageResults.push(item);
      } else {
        batchFailures++;
        failedPrs.push({
          prNumber: batchTargets[j] && batchTargets[j].number ? batchTargets[j].number : -1,
          reason: "diagnostic_agent_failed_or_schema_validation_failed"
        });
      }
    }

    if (batchFailures === batch.length) {
      consecutiveFailedBatches++;
    } else {
      consecutiveFailedBatches = 0;
    }

    if (failedPrs.length >= config.failureTolerance) {
      log(`EARLY EXIT: Diagnostic failure tolerance reached (${failedPrs.length}/${config.failureTolerance}).`);
      break;
    }

    if (consecutiveFailedBatches >= 2) {
      log("EARLY EXIT: Two consecutive diagnostic batches failed completely.");
      break;
    }

    if (budgetTotal() > config.tokenSafetyCeiling) {
      log(`EARLY EXIT: Token budget ceiling exceeded mid-run (${budgetTotal()} > ${config.tokenSafetyCeiling}).`);
      break;
    }
  }

  return {
    triageResults,
    failedPrs
  };
}

async function runEvidenceAudit(inventory, triageResults, failedPrs, config) {
  const prompt = `You are an independent release safety reviewer. Audit these PR diagnostic results before merge queue synthesis.

TARGET BASE:
${config.targetBase}

FIX MODE:
${config.fixMode}

ORIGINAL INVENTORY:
${JSON.stringify(inventory.prs, null, 2)}

TRIAGE RESULTS:
${JSON.stringify(triageResults, null, 2)}

FAILED DIAGNOSTICS:
${JSON.stringify(failedPrs, null, 2)}

AUDIT RUBRIC:
1. Evidence must support the failureClass and rootCause.
2. clean_ready requires passing checks, mergeability, non-draft state, and no policy/review blocker.
3. Any PR with missing verification commands must go to needsRecheck.
4. Any PR with non-empty doNotMergeIf cannot be ready_to_merge.
5. Any apply-mode patch with failing or partial verification must be blocked or human-review required.
6. Flag proposed fixes that look like symptom patches rather than root-cause fixes.
7. Do not overrule evidence with optimism.

You may run lightweight gh read-only spot checks for suspicious entries, but do not edit files.

Return only the JSON object matching the schema.`;

  return await agent(prompt, buildAgentOptions(
    "evidence-audit",
    REVIEW_MODEL,
    evidenceAuditSchema,
    false
  ));
}

async function runSynthesis(inventory, triageResults, failedPrs, evidenceAudit, config) {
  const prompt = `You are the principal release engineer. Produce the final PR rescue report and safe merge queue.

BASE BRANCH:
${config.targetBase}

FIX MODE:
${config.fixMode}

ORIGINAL INVENTORY:
${JSON.stringify(inventory.prs, null, 2)}

TRIAGE RESULTS:
${JSON.stringify(triageResults, null, 2)}

FAILED DIAGNOSTICS:
${JSON.stringify(failedPrs, null, 2)}

EVIDENCE AUDIT:
${JSON.stringify(evidenceAudit, null, 2)}

RANKING RULES:
1. ready_to_merge:
   - clean_ready
   - low mergeRisk
   - no doNotMergeIf
   - evidence audit approved or audit disabled with strong direct evidence
2. verify_then_merge:
   - likely safe, but one explicit verification command must be run first.
3. fix_required:
   - minimal fix exists but has not been safely applied or verified.
4. human_review_required:
   - review/policy/security/domain uncertainty blocks automation.
5. blocked:
   - high risk, failed verification, conflict, missing evidence, unsafe apply result, or explicit doNotMergeIf.

SAFETY SCORE:
- 95-100: trivial, verified, clean-ready.
- 80-94: low risk, needs one verification.
- 60-79: fix likely but work remains.
- 30-59: substantial uncertainty or human review needed.
- 0-29: blocked/high risk.

OUTPUT REQUIREMENTS:
- Include every analyzed PR in either safeMergeQueue or blockedPRs.
- Include failed diagnostics in summaryStats.failedDiagnostics and remediationPlaybook.
- Keep suggestedNextAction concrete.
- Include markdownReport that humans can paste into a release channel.
- Never recommend merge for draft PRs, conflicted PRs, failed verification, high-risk PRs, or PRs with non-empty doNotMergeIf.

Return only the JSON object matching the schema.`;

  return await agent(prompt, buildAgentOptions(
    "synthesize-queue",
    REVIEW_MODEL,
    synthesisSchema,
    false
  ));
}

async function executePrRescue() {
  const a = runtimeArgs();

  const targetBase = sanitizeText(a.baseBranch || "main", "main");
  const fixMode = normalizeFixMode(a.fixMode);
  const prScope = normalizePrInput(a.prs || "open");

  const config = {
    targetBase,
    fixMode,
    prScope,
    maxConcurrent: clampInteger(a.maxConcurrent, 1, HARD_CONCURRENCY_CAP, DEFAULT_MAX_CONCURRENT),
    maxPRs: clampInteger(a.maxPRs, 1, 100, DEFAULT_MAX_PRS),
    tokenSafetyCeiling: clampInteger(a.tokenSafetyCeiling, 100000, 20000000, DEFAULT_TOKEN_SAFETY_CEILING),
    warningTokenThreshold: clampInteger(a.warningTokenThreshold, 50000, 20000000, DEFAULT_WARNING_TOKEN_THRESHOLD),
    dryRun: toBoolean(a.dryRun, false),
    allowOverBudget: toBoolean(a.allowOverBudget, false),
    auditEvidence: toBoolean(a.auditEvidence, true),
    includeRawTriage: toBoolean(a.includeRawTriage, false),
    includeInventory: toBoolean(a.includeInventory, false),
    runId: sanitizeText(a.runId || "manual-run", "manual-run")
  };

  if (!isSafeBranchName(config.targetBase)) {
    return {
      status: "ABORTED",
      reason: "invalid_base_branch",
      details: "baseBranch may contain only letters, numbers, '.', '_', '/', and '-'."
    };
  }

  if (config.prScope.mode === "explicit" && config.prScope.numbers.length === 0) {
    return {
      status: "ABORTED",
      reason: "invalid_pr_scope",
      details: "args.prs must be 'open', a PR number, a comma-separated PR list, or an array of positive PR numbers."
    };
  }

  if (config.fixMode === "apply" && toBoolean(a.confirmApplyMode, false) !== true) {
    return {
      status: "ABORTED",
      reason: "apply_mode_requires_confirmApplyMode",
      details: "Set args.confirmApplyMode=true to allow isolated worktree patch attempts. The workflow still will not push, merge, close, approve, or commit."
    };
  }

  if (budgetTotal() > config.tokenSafetyCeiling) {
    log(`ABORT: Existing session token spend (${budgetTotal()}) exceeds ceiling (${config.tokenSafetyCeiling}).`);
    return {
      status: "ABORTED",
      reason: "token_budget_exceeded_preflight",
      budgetTotal: budgetTotal(),
      tokenSafetyCeiling: config.tokenSafetyCeiling
    };
  }

  phase("Preflight & Scope Validation");
  log(`Starting PR Rescue Factory. base='${config.targetBase}', scope='${config.prScope.mode}', fixMode='${config.fixMode}', maxConcurrent=${config.maxConcurrent}.`);

  const preflight = await runPreflight(config);

  if (!preflight || preflight.ok !== true) {
    log("Preflight failed. Aborting before inventory and fan-out.");
    return {
      status: "ABORTED",
      reason: "preflight_failed",
      preflight
    };
  }

  phase("PR Inventory & Classification");

  const inventory = await runInventory(config);

  if (!inventory || !inventory.prs || inventory.prs.length === 0) {
    log("No PRs found in scope. Workflow complete.");
    return {
      status: "COMPLETED",
      reason: "no_prs_in_scope",
      preflight,
      queue: []
    };
  }

  let targets = inventory.prs;

  if (config.prScope.mode === "explicit") {
    const allowed = {};
    for (let i = 0; i < config.prScope.numbers.length; i++) {
      allowed[String(config.prScope.numbers[i])] = true;
    }

    const filtered = [];
    for (let j = 0; j < targets.length; j++) {
      if (allowed[String(targets[j].number)]) {
        filtered.push(targets[j]);
      }
    }
    targets = filtered;
  }

  if (targets.length > config.maxPRs) {
    targets = targets.slice(0, config.maxPRs);
  }

  if (targets.length === 0) {
    return {
      status: "COMPLETED",
      reason: "no_matching_prs_after_filter",
      inventory
    };
  }

  config.failureTolerance = clampInteger(
    a.failureTolerance,
    1,
    targets.length,
    Math.max(1, Math.floor(targets.length * 0.25))
  );

  const projectedTokens = estimatedTokenCost(targets.length, config.fixMode, config.auditEvidence);

  log(`Inventory resolved ${targets.length} PR(s). Estimated workflow cost: ~${projectedTokens} tokens. Current session budget: ${budgetTotal()}.`);

  if (!config.allowOverBudget && currentBudgetWouldExceed(projectedTokens, config.tokenSafetyCeiling)) {
    return {
      status: "ABORTED",
      reason: "projected_token_budget_exceeded",
      projectedTokens,
      currentBudgetTotal: budgetTotal(),
      tokenSafetyCeiling: config.tokenSafetyCeiling,
      recommendation: "Pass a smaller args.prs subset, lower args.maxPRs, or explicitly set args.allowOverBudget=true after human review."
    };
  }

  if (projectedTokens > config.warningTokenThreshold) {
    log(`WARNING: Projected tokens (${projectedTokens}) exceed warning threshold (${config.warningTokenThreshold}).`);
  }

  if (config.dryRun) {
    log("Dry run requested. Stopping after preflight, inventory, and cost projection.");
    return {
      status: "DRY_RUN_COMPLETED",
      runId: config.runId,
      fixModeUsed: config.fixMode,
      baseBranch: config.targetBase,
      targetPrNumbers: targets.map((p) => p.number),
      projectedTokens,
      maxConcurrent: config.maxConcurrent,
      failureTolerance: config.failureTolerance,
      preflight,
      inventory
    };
  }

  phase("Parallel Diagnostic & Fix Proposal");
  log(`Fanning out diagnostics for ${targets.length} PR(s) in batches of ${config.maxConcurrent}.`);

  const triageBundle = await runTriageBatches(targets, config);
  const validTriage = triageBundle.triageResults.filter(Boolean);
  const failedPrs = triageBundle.failedPrs;

  log(`Completed diagnostics: ${validTriage.length}/${targets.length} valid result(s), ${failedPrs.length} failed diagnostic(s).`);

  if (validTriage.length === 0) {
    return {
      status: "FAILED",
      reason: "all_triage_agents_failed_validation",
      runId: config.runId,
      failedPrs,
      inventory: config.includeInventory ? inventory : undefined
    };
  }

  phase("Evidence Audit & Merge Gate");

  let evidenceAudit = {
    overallEvidenceQuality: "mixed",
    approvedPrNumbers: [],
    needsRecheck: [],
    unsafeMergeSuggestions: [],
    suspectedSymptomPatches: [],
    globalWarnings: ["Evidence audit disabled by args.auditEvidence=false."]
  };

  if (config.auditEvidence) {
    evidenceAudit = await runEvidenceAudit(inventory, validTriage, failedPrs, config);
  } else {
    log("Evidence audit disabled. Synthesis will treat unaudited entries conservatively.");
  }

  phase("Risk Synthesis & Safe Merge Queue");
  log("Synthesizing final safe merge queue and remediation playbook.");

  const finalReport = await runSynthesis(inventory, validTriage, failedPrs, evidenceAudit, config);

  const output = {
    status: "COMPLETED",
    runId: config.runId,
    baseBranch: config.targetBase,
    fixModeUsed: config.fixMode,
    analyzedPrNumbers: validTriage.map((r) => r.prNumber),
    failedPrs,
    projectedTokens,
    evidenceAudit,
    ...finalReport
  };

  if (config.includeRawTriage) {
    output.rawTriageResults = validTriage;
  }

  if (config.includeInventory) {
    output.inventory = inventory;
  }

  log("Workflow complete. Safe merge queue, blocked list, and remediation playbook are ready for human review.");

  return output;
}

return await executePrRescue();