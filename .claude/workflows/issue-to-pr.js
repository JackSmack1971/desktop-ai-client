'use strict';

/**
 * issue-to-pr.js
 *
 * Hardened workflow contract for implementing one GitHub issue and producing
 * PR-ready evidence.
 *
 * Usage:
 *   node issue-to-pr.js https://github.com/org/repo/issues/123
 *   node issue-to-pr.js org/repo#123 --max-files=8 --max-test-attempts=2
 *   node issue-to-pr.js "#123 fix login redirect" --format=markdown
 */

const WORKFLOW_NAME = 'issue-to-pr';
const WORKFLOW_VERSION = 2;

const DEFAULT_LIMITS = Object.freeze({
  maxFilesChanged: 10,
  maxTestAttempts: 2,
  maxImplementationPasses: 2,
  requireCleanDiffBeforeStart: true,
  requireIndependentReview: true,
  requireRollbackNotes: true
});

const OUTPUT_SECTIONS = Object.freeze([
  'scope',
  'issue-summary',
  'implementation-plan',
  'files-changed',
  'verification-results',
  'privacy-provider-storage-impact',
  'open-risks',
  'rollback-notes',
  'pr-description'
]);

const RISK_SURFACES = Object.freeze([
  {
    id: 'privacy',
    label: 'privacy and secret exposure',
    checks: [
      'No secrets, tokens, credentials, private URLs, customer data, or local environment values are added to code, logs, tests, snapshots, or PR text.',
      'New logging does not expose user content, provider payloads, auth headers, cookies, or internal identifiers beyond existing safe conventions.'
    ]
  },
  {
    id: 'provider-routing',
    label: 'provider routing drift',
    checks: [
      'Provider/model routing behavior is unchanged unless the issue explicitly requires it.',
      'Fallbacks, retries, timeouts, and provider-specific options remain compatible with existing behavior.'
    ]
  },
  {
    id: 'storage',
    label: 'storage migration or retention changes',
    checks: [
      'No schema, migration, retention, cache, queue, or persistence behavior changes are introduced unless explicitly in scope.',
      'Any storage change includes rollback notes, migration safety notes, and targeted verification.'
    ]
  },
  {
    id: 'telemetry-release-evidence',
    label: 'telemetry or release evidence leakage',
    checks: [
      'Telemetry, analytics, traces, release notes, screenshots, and PR evidence do not leak sensitive data.',
      'Evidence is specific enough for reviewers without exposing protected data.'
    ]
  }
]);

const AGENTS = Object.freeze([
  {
    id: 'issue-context-agent',
    role: 'Read the issue, repository instructions, nearby AGENTS/CLAUDE docs, and relevant code paths. Return only scoped facts, acceptance criteria, and unknowns.',
    allowedActions: ['read', 'search', 'summarize'],
    forbiddenActions: ['edit', 'commit', 'push']
  },
  {
    id: 'implementation-agent',
    role: 'Implement the smallest correct change that satisfies the issue and repository instructions.',
    allowedActions: ['read', 'edit', 'run targeted checks'],
    forbiddenActions: [
      'broad rewrites',
      'unrequested refactors',
      'dependency upgrades unless required by the issue',
      'schema or retention changes unless required by the issue'
    ]
  },
  {
    id: 'pr-reviewer',
    role: 'Independently review the final diff against scope, regressions, privacy, provider routing, storage impact, and PR readiness.',
    allowedActions: ['read diff', 'run or inspect targeted verification', 'produce review findings'],
    forbiddenActions: ['approve without evidence', 'review from implementation memory only']
  }
]);

const SKILLS = Object.freeze([
  'stack-detection',
  'repo-instruction-reading',
  'bounded-implementation',
  'privacy-boundary-review',
  'provider-routing-review',
  'storage-recovery-review',
  'targeted-verification',
  'pr-evidence-capture'
]);

const PHASES = Object.freeze([
  {
    id: 'preflight',
    title: 'Preflight',
    objective: 'Confirm input, repo state, instructions, and safe scope before editing.',
    requiredEvidence: [
      'Issue reference is parsed or explicitly treated as free-form scope.',
      'Repository instructions and nearest AGENTS/CLAUDE guidance are inspected.',
      'Working tree state is checked before mutation.'
    ],
    exitGate: 'Do not edit until scope, relevant instructions, and likely verification path are known.'
  },
  {
    id: 'issue-analysis',
    title: 'Issue analysis',
    objective: 'Extract acceptance criteria and narrow the implementation surface.',
    requiredEvidence: [
      'Summarize the requested behavior in one or two sentences.',
      'List concrete acceptance criteria.',
      'Identify files or modules likely to change.',
      'Record out-of-scope items.'
    ],
    exitGate: 'Proceed only when the smallest viable change is clear.'
  },
  {
    id: 'implementation',
    title: 'Implementation',
    objective: 'Make the smallest correct code/test/docs change.',
    requiredEvidence: [
      'Keep changes narrowly tied to the issue.',
      'Avoid unrelated cleanup.',
      'Prefer existing project patterns over new abstractions.',
      'Add or adjust tests only where they prove the issue behavior.'
    ],
    exitGate: 'Stop after the minimal passing change; do not expand scope.'
  },
  {
    id: 'targeted-verification',
    title: 'Targeted verification',
    objective: 'Run the narrowest reliable checks that prove the change.',
    requiredEvidence: [
      'Show exact commands run.',
      'Capture pass/fail result for each command.',
      'Explain any skipped verification with a concrete reason.',
      'Run post-mutation state checks such as git diff and status.'
    ],
    exitGate: 'If verification fails, perform at most the configured number of fix attempts before reporting open risk.'
  },
  {
    id: 'independent-review',
    title: 'Independent review',
    objective: 'Have a separate reviewer inspect diff, evidence, and risk surfaces.',
    requiredEvidence: [
      'Review final diff against issue acceptance criteria.',
      'Check privacy, provider routing, storage, telemetry, and release-evidence surfaces.',
      'Identify regressions, missing tests, or PR blockers.',
      'State approved, approved-with-risks, or rejected.'
    ],
    exitGate: 'Do not prepare PR output until review findings are resolved or clearly documented.'
  },
  {
    id: 'pr-packaging',
    title: 'PR packaging',
    objective: 'Produce reviewer-ready PR text and rollback notes.',
    requiredEvidence: [
      'Files changed summary.',
      'Verification commands and results.',
      'Risk impact section.',
      'Rollback notes.',
      'Open risks or follow-ups.'
    ],
    exitGate: 'PR output must be complete enough to paste into /create:pr.'
  }
]);

function normalizeText(value) {
  return String(value == null ? '' : value)
    .replace(/\s+/g, ' ')
    .trim();
}

function parsePositiveInt(value, fallback, optionName) {
  if (value == null || value === '') return fallback;

  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0) {
    throw new Error(`Invalid ${optionName}: expected a non-negative integer, got "${value}".`);
  }

  return parsed;
}

function unique(values) {
  return Array.from(new Set(values.filter(Boolean)));
}

function slugify(value, maxLength = 56) {
  const slug = normalizeText(value)
    .toLowerCase()
    .replace(/https?:\/\//g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, maxLength)
    .replace(/-+$/g, '');

  return slug || 'issue';
}

function parseIssueReference(rawScope) {
  const scope = normalizeText(rawScope);

  const base = {
    raw: scope || null,
    kind: scope ? 'free-form' : 'missing',
    provider: null,
    owner: null,
    repo: null,
    number: null,
    url: null,
    normalized: null
  };

  if (!scope) return base;

  const githubUrl = scope.match(
    /https?:\/\/(?:www\.)?github\.com\/([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+)\/issues\/(\d+)(?:[/?#][^\s]*)?/i
  );

  if (githubUrl) {
    const owner = githubUrl[1];
    const repo = githubUrl[2];
    const number = Number(githubUrl[3]);

    return {
      ...base,
      kind: 'github-issue-url',
      provider: 'github',
      owner,
      repo,
      number,
      url: `https://github.com/${owner}/${repo}/issues/${number}`,
      normalized: `${owner}/${repo}#${number}`
    };
  }

  const shorthand = scope.match(
    /(?:^|\s)([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+)#(\d+)(?:\s|$)/
  );

  if (shorthand) {
    const owner = shorthand[1];
    const repo = shorthand[2];
    const number = Number(shorthand[3]);

    return {
      ...base,
      kind: 'github-shorthand',
      provider: 'github',
      owner,
      repo,
      number,
      url: `https://github.com/${owner}/${repo}/issues/${number}`,
      normalized: `${owner}/${repo}#${number}`
    };
  }

  const numberOnly = scope.match(/^(?:issue\s*)?#?(\d+)$/i);

  if (numberOnly) {
    const number = Number(numberOnly[1]);

    return {
      ...base,
      kind: 'issue-number',
      provider: 'github',
      number,
      normalized: `#${number}`
    };
  }

  return {
    ...base,
    normalized: scope
  };
}

function buildBranchName(issue, scope) {
  if (issue && issue.owner && issue.repo && issue.number) {
    return `issue-to-pr/${slugify(issue.owner, 24)}-${slugify(issue.repo, 24)}-${issue.number}`;
  }

  if (issue && issue.number) {
    return `issue-to-pr/issue-${issue.number}`;
  }

  return `issue-to-pr/${slugify(scope)}`;
}

function inferRiskProfile(scope) {
  const text = normalizeText(scope).toLowerCase();

  const matched = [];

  const rules = [
    {
      id: 'privacy',
      terms: [
        'auth',
        'token',
        'secret',
        'password',
        'cookie',
        'session',
        'pii',
        'privacy',
        'credential',
        'log'
      ]
    },
    {
      id: 'provider-routing',
      terms: [
        'provider',
        'model',
        'routing',
        'fallback',
        'openai',
        'anthropic',
        'vertex',
        'bedrock',
        'llm'
      ]
    },
    {
      id: 'storage',
      terms: [
        'database',
        'migration',
        'schema',
        'retention',
        'cache',
        'queue',
        'redis',
        'postgres',
        's3'
      ]
    },
    {
      id: 'telemetry-release-evidence',
      terms: [
        'telemetry',
        'analytics',
        'trace',
        'release',
        'screenshot',
        'evidence',
        'observability'
      ]
    }
  ];

  for (const rule of rules) {
    if (rule.terms.some((term) => text.includes(term))) {
      matched.push(rule.id);
    }
  }

  return {
    level: matched.length === 0 ? 'standard' : matched.length >= 3 ? 'high' : 'elevated',
    matchedSurfaces: unique(matched),
    requiredReviewSurfaces: matched.length === 0
      ? RISK_SURFACES.map((surface) => surface.id)
      : unique([...matched, 'privacy'])
  };
}

function buildVerificationPlan(options) {
  return [
    {
      id: 'diff-review',
      command: 'git diff --check && git diff',
      purpose: 'Inspect exact mutations and whitespace safety.',
      required: true
    },
    {
      id: 'status-review',
      command: 'git status --short',
      purpose: 'Confirm only intended files changed.',
      required: true
    },
    {
      id: 'targeted-tests',
      command: '<repo-specific targeted test command>',
      purpose: 'Run the narrowest tests that prove the issue behavior.',
      required: true,
      maxAttempts: options.maxTestAttempts
    },
    {
      id: 'lint-or-typecheck',
      command: '<repo-specific lint/typecheck command when relevant>',
      purpose: 'Catch integration or style regressions near the changed surface.',
      required: false
    },
    {
      id: 'manual-inspection',
      command: '<manual or static inspection notes>',
      purpose: 'Use when runtime tests are unavailable or disproportionate.',
      required: false
    }
  ];
}

function buildPrTemplate() {
  return {
    titleFormat: '<concise issue-oriented title>',
    bodySections: [
      {
        title: 'Summary',
        prompt: 'What changed and why?'
      },
      {
        title: 'Issue',
        prompt: 'Link the issue reference.'
      },
      {
        title: 'Changes',
        prompt: 'List the important files or behavior changes.'
      },
      {
        title: 'Verification',
        prompt: 'Paste exact commands and results.'
      },
      {
        title: 'Risk / Impact',
        prompt: 'State privacy, provider routing, storage, telemetry, and release-evidence impact.'
      },
      {
        title: 'Rollback',
        prompt: 'Explain how to revert safely.'
      },
      {
        title: 'Open Risks',
        prompt: 'List known limitations, skipped checks, or follow-ups.'
      }
    ]
  };
}

function buildContract(rawScope, rawOptions = {}) {
  const scope = normalizeText(rawScope);
  const issue = parseIssueReference(scope);

  const options = {
    maxFilesChanged: parsePositiveInt(
      rawOptions.maxFilesChanged,
      DEFAULT_LIMITS.maxFilesChanged,
      '--max-files'
    ),
    maxTestAttempts: parsePositiveInt(
      rawOptions.maxTestAttempts,
      DEFAULT_LIMITS.maxTestAttempts,
      '--max-test-attempts'
    ),
    maxImplementationPasses: parsePositiveInt(
      rawOptions.maxImplementationPasses,
      DEFAULT_LIMITS.maxImplementationPasses,
      '--max-implementation-passes'
    ),
    strict: rawOptions.strict !== false,
    format: rawOptions.format || 'json'
  };

  const hasInput = scope.length > 0;
  const riskProfile = inferRiskProfile(scope);

  const contract = {
    status: hasInput ? 'active-contract' : 'needs-input',
    workflow: WORKFLOW_NAME,
    version: WORKFLOW_VERSION,
    intent: 'Implement exactly one issue with bounded scope, explicit verification, independent review, and PR-ready evidence.',
    requiredInput: {
      name: 'issueUrlOrNumber',
      acceptedFormats: [
        'https://github.com/<owner>/<repo>/issues/<number>',
        '<owner>/<repo>#<number>',
        '#<number>',
        '<free-form issue scope>'
      ]
    },
    scope: hasInput ? scope : null,
    parsedIssue: hasInput ? issue : null,
    suggestedBranch: hasInput ? buildBranchName(issue, scope) : null,
    relatedCommands: ['/create:pr'],
    limits: {
      maxFilesChanged: options.maxFilesChanged,
      maxTestAttempts: options.maxTestAttempts,
      maxImplementationPasses: options.maxImplementationPasses,
      strictScope: options.strict,
      requireCleanDiffBeforeStart: DEFAULT_LIMITS.requireCleanDiffBeforeStart,
      requireIndependentReview: DEFAULT_LIMITS.requireIndependentReview,
      requireRollbackNotes: DEFAULT_LIMITS.requireRollbackNotes
    },
    operatingRules: [
      'Implement one issue only.',
      'Prefer the smallest correct change over broad refactoring.',
      'Do not change public behavior beyond the issue acceptance criteria.',
      'Do not add dependencies unless the issue cannot be solved safely without them.',
      'Do not alter provider routing, privacy boundaries, storage behavior, telemetry, or release evidence unless explicitly required.',
      'Treat missing tests or skipped checks as open risk, not as success.',
      'The implementation agent must not be the only reviewer of its own changes.'
    ],
    phaseOrder: PHASES.map((phase) => phase.id),
    phases: PHASES,
    agents: AGENTS,
    skills: SKILLS,
    riskProfile,
    riskSurfaces: RISK_SURFACES,
    verification: buildVerificationPlan(options),
    reviewGate: {
      required: true,
      reviewer: 'pr-reviewer',
      decisionValues: ['approved', 'approved-with-risks', 'rejected'],
      approvalCriteria: [
        'Diff satisfies issue acceptance criteria.',
        'Changed files are within the expected scope.',
        'Verification evidence is present and credible.',
        'Privacy/provider/storage/telemetry risks are either absent or documented.',
        'Rollback notes are actionable.',
        'No obvious unrelated cleanup or opportunistic refactor is included.'
      ],
      rejectionCriteria: [
        'Issue scope is ambiguous and implementation guesses behavior.',
        'Diff includes unrelated broad rewrites.',
        'Tests fail without documented reason and follow-up.',
        'Sensitive data could be exposed.',
        'Storage/provider/telemetry behavior changed without explicit issue requirement.'
      ]
    },
    outputSections: OUTPUT_SECTIONS,
    prTemplate: buildPrTemplate(),
    rollbackRequirements: [
      'State whether the change can be reverted with a normal git revert.',
      'Mention any data, cache, migration, config, or deployment rollback steps.',
      'Call out irreversible or manually verified side effects.'
    ],
    completionDefinition: {
      doneWhen: [
        'Issue scope is satisfied.',
        'Targeted verification has been run or skipped with a concrete reason.',
        'Independent review gate is passed or risks are explicitly documented.',
        'PR body contains summary, tests, risks, and rollback notes.'
      ],
      notDoneWhen: [
        'The implementation depends on unstated assumptions.',
        'The diff contains unrelated refactors.',
        'Verification evidence is missing.',
        'Risk surfaces are not reviewed.'
      ]
    }
  };

  contract.validation = validateContract(contract);

  return contract;
}

function validateContract(contract) {
  const errors = [];
  const warnings = [];

  if (!contract.scope) {
    errors.push('Missing issueUrlOrNumber input.');
  }

  if (!Array.isArray(contract.phases) || contract.phases.length === 0) {
    errors.push('Workflow must define at least one phase.');
  }

  if (!Array.isArray(contract.agents) || contract.agents.length < 2) {
    errors.push('Workflow must define implementation and independent review agents.');
  }

  if (!contract.reviewGate || contract.reviewGate.required !== true) {
    errors.push('Independent review gate is required.');
  }

  if (
    contract.limits &&
    contract.limits.maxFilesChanged > DEFAULT_LIMITS.maxFilesChanged
  ) {
    warnings.push(
      `maxFilesChanged is above the default guardrail of ${DEFAULT_LIMITS.maxFilesChanged}.`
    );
  }

  if (
    contract.limits &&
    contract.limits.maxImplementationPasses > DEFAULT_LIMITS.maxImplementationPasses
  ) {
    warnings.push(
      `maxImplementationPasses is above the default guardrail of ${DEFAULT_LIMITS.maxImplementationPasses}.`
    );
  }

  return {
    ok: errors.length === 0,
    errors,
    warnings
  };
}

function emitContract(contract, writer = process.stdout) {
  writer.write(`${JSON.stringify(contract, null, 2)}\n`);
}

function formatMarkdown(contract) {
  if (contract.status === 'needs-input') {
    return [
      `# ${contract.workflow} v${contract.version}`,
      '',
      '**Status:** needs input',
      '',
      `Required input: \`${contract.requiredInput.name}\``,
      '',
      'Accepted formats:',
      ...contract.requiredInput.acceptedFormats.map((format) => `- \`${format}\``)
    ].join('\n');
  }

  const lines = [
    `# ${contract.workflow} v${contract.version}`,
    '',
    `**Status:** ${contract.status}`,
    `**Scope:** ${contract.scope}`,
    `**Parsed issue:** ${contract.parsedIssue.normalized || contract.parsedIssue.raw}`,
    `**Suggested branch:** \`${contract.suggestedBranch}\``,
    `**Risk level:** ${contract.riskProfile.level}`,
    '',
    '## Operating rules',
    ...contract.operatingRules.map((rule) => `- ${rule}`),
    '',
    '## Phases',
    ...contract.phases.map((phase, index) => [
      `### ${index + 1}. ${phase.title}`,
      phase.objective,
      '',
      '**Required evidence:**',
      ...phase.requiredEvidence.map((item) => `- ${item}`),
      '',
      `**Exit gate:** ${phase.exitGate}`
    ].join('\n')),
    '',
    '## Verification',
    ...contract.verification.map((item) => [
      `- **${item.id}**`,
      `  - Command: \`${item.command}\``,
      `  - Purpose: ${item.purpose}`,
      `  - Required: ${item.required ? 'yes' : 'no'}`
    ].join('\n')),
    '',
    '## PR output sections',
    ...contract.outputSections.map((section) => `- ${section}`)
  ];

  return lines.join('\n');
}

function parseCli(argv) {
  const scopeParts = [];
  const options = {};

  for (const arg of argv) {
    if (arg === '--loose') {
      options.strict = false;
      continue;
    }

    if (arg === '--strict') {
      options.strict = true;
      continue;
    }

    if (arg.startsWith('--max-files=')) {
      options.maxFilesChanged = arg.slice('--max-files='.length);
      continue;
    }

    if (arg.startsWith('--max-test-attempts=')) {
      options.maxTestAttempts = arg.slice('--max-test-attempts='.length);
      continue;
    }

    if (arg.startsWith('--max-implementation-passes=')) {
      options.maxImplementationPasses = arg.slice('--max-implementation-passes='.length);
      continue;
    }

    if (arg.startsWith('--format=')) {
      const format = arg.slice('--format='.length);
      if (!['json', 'markdown'].includes(format)) {
        throw new Error(`Invalid --format value: "${format}". Expected "json" or "markdown".`);
      }
      options.format = format;
      continue;
    }

    scopeParts.push(arg);
  }

  return {
    scope: scopeParts.join(' '),
    options
  };
}

if (require.main === module) {
  try {
    const { scope, options } = parseCli(process.argv.slice(2));
    const contract = buildContract(scope, options);

    if (options.format === 'markdown') {
      process.stdout.write(`${formatMarkdown(contract)}\n`);
    } else {
      emitContract(contract);
    }

    if (!contract.validation.ok) {
      process.exitCode = 1;
    }
  } catch (error) {
    process.stderr.write(`issue-to-pr workflow error: ${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  WORKFLOW_NAME,
  WORKFLOW_VERSION,
  DEFAULT_LIMITS,
  OUTPUT_SECTIONS,
  RISK_SURFACES,
  AGENTS,
  SKILLS,
  PHASES,
  buildContract,
  buildBranchName,
  emitContract,
  formatMarkdown,
  inferRiskProfile,
  parseCli,
  parseIssueReference,
  validateContract
};