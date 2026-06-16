'use strict';

const WORKFLOW_NAME = 'release-readiness';
const WORKFLOW_VERSION = 2;
const RELATED_COMMANDS = Object.freeze(['/release:readiness']);

const DEFAULT_OPTIONS = Object.freeze({
  strict: true,
  includeAgentPrompts: true,
  failOnUnknowns: true,
  maxEvidenceGapsForConditional: 2
});

const meta = Object.freeze({
  name: WORKFLOW_NAME,
  description: 'Assess packaging, privacy, provider, storage, telemetry, and automation readiness before build, publish, or tagged release.',
  whenToUse: 'Run before any release candidate, publish dry-run, production build, or tag cut.',
  phases: Object.freeze([
    { title: 'Scope Validation', detail: 'Confirm branch, build target, release type, and approval boundary.' },
    { title: 'Inventory Discovery', detail: 'Find release commands, package manifests, CI jobs, and release automation.' },
    { title: 'Boundary Evidence Review', detail: 'Review privacy, provider routing, storage, migration, and telemetry evidence.' },
    { title: 'Packaging Surface Review', detail: 'Inspect artifact contents, versioning, changelog, dependency, and publish surfaces.' },
    { title: 'Adversarial Gate', detail: 'Separate blockers from warnings and reject approval when evidence is missing.' },
    { title: 'Verdict Synthesis', detail: 'Return a clear release verdict, evidence map, blockers, and safe next step.' }
  ])
});

const AGENTS = Object.freeze([
  {
    id: 'release-scope-controller',
    role: 'Validate the release target and constrain the review to the requested branch, tag, package, or build target.',
    skills: Object.freeze(['release-evidence-review'])
  },
  {
    id: 'command-inventory-auditor',
    role: 'Discover build, test, lint, package, publish, and release automation commands without executing destructive operations.',
    skills: Object.freeze(['release-evidence-review'])
  },
  {
    id: 'boundary-risk-auditor',
    role: 'Audit privacy, provider routing, storage, migration, telemetry, and secret-handling boundaries.',
    skills: Object.freeze(['privacy-boundary-review', 'provider-routing-review', 'storage-recovery-review'])
  },
  {
    id: 'packaging-surface-auditor',
    role: 'Inspect package metadata, publish files, artifact contents, dependency exposure, changelog, version, and tag readiness.',
    skills: Object.freeze(['release-evidence-review'])
  },
  {
    id: 'release-gatekeeper',
    role: 'Synthesize evidence, challenge optimistic assumptions, classify blockers, and produce the final verdict.',
    skills: Object.freeze([
      'release-evidence-review',
      'privacy-boundary-review',
      'provider-routing-review',
      'storage-recovery-review'
    ])
  }
]);

const RISK_SURFACES = Object.freeze([
  {
    id: 'command-exposure',
    label: 'command exposure',
    blockerWhen: 'Release, publish, deploy, or migration commands can run without explicit target, dry-run, or approval boundaries.'
  },
  {
    id: 'privacy-leakage',
    label: 'privacy leakage',
    blockerWhen: 'Artifacts, logs, telemetry, or package files can expose secrets, user data, internal prompts, traces, or private configuration.'
  },
  {
    id: 'provider-routing-drift',
    label: 'provider routing drift',
    blockerWhen: 'Runtime provider/model routing differs from documented release assumptions or lacks fallback behavior.'
  },
  {
    id: 'storage-migration-risk',
    label: 'storage or migration risk',
    blockerWhen: 'Schema, storage, cache, or persistence changes lack migration, rollback, or recovery evidence.'
  },
  {
    id: 'telemetry-leakage',
    label: 'telemetry leakage',
    blockerWhen: 'Analytics, diagnostics, or traces can emit sensitive payloads or cannot be disabled where required.'
  },
  {
    id: 'artifact-integrity',
    label: 'artifact integrity',
    blockerWhen: 'Build output, package contents, checksums, provenance, or generated files are unknown or unreviewed.'
  },
  {
    id: 'dependency-and-license-risk',
    label: 'dependency and license risk',
    blockerWhen: 'New or changed dependencies lack vulnerability, license, or lockfile review.'
  },
  {
    id: 'release-automation-risk',
    label: 'unreviewed release automation',
    blockerWhen: 'CI, tagging, changelog, versioning, or publish automation changed without review or dry-run evidence.'
  }
]);

const EVIDENCE_REQUIREMENTS = Object.freeze([
  {
    id: 'target-scope',
    required: true,
    ownerAgent: 'release-scope-controller',
    description: 'Requested branch, tag, package, or build target is explicit and matches the release intent.',
    examples: Object.freeze(['current git branch', 'target tag/version', 'package or app name', 'release channel'])
  },
  {
    id: 'command-inventory',
    required: true,
    ownerAgent: 'command-inventory-auditor',
    description: 'Build, test, lint, package, publish, migration, and release commands are discovered and classified by risk.',
    examples: Object.freeze(['package scripts', 'Makefile targets', 'CI workflow jobs', 'release scripts'])
  },
  {
    id: 'quality-signal',
    required: true,
    ownerAgent: 'command-inventory-auditor',
    description: 'Tests, linting, type checks, build checks, and dry-run commands are identified with pass/fail or missing status.',
    examples: Object.freeze(['test result', 'lint result', 'typecheck result', 'build result', 'publish dry-run result'])
  },
  {
    id: 'privacy-boundary',
    required: true,
    ownerAgent: 'boundary-risk-auditor',
    description: 'Artifacts, logs, telemetry, and generated files are checked for secrets, PII, prompts, traces, and private configuration.',
    examples: Object.freeze(['.npmignore/files allowlist', 'env access review', 'log redaction review', 'secret scan'])
  },
  {
    id: 'provider-routing',
    required: true,
    ownerAgent: 'boundary-risk-auditor',
    description: 'Provider, model, API, region, failover, and feature-flag routing are consistent with release expectations.',
    examples: Object.freeze(['provider config', 'feature flags', 'fallback behavior', 'region assumptions'])
  },
  {
    id: 'storage-and-recovery',
    required: true,
    ownerAgent: 'boundary-risk-auditor',
    description: 'Storage, schema, cache, migration, and rollback paths are known and safe for this release.',
    examples: Object.freeze(['migration files', 'rollback notes', 'backup procedure', 'data compatibility note'])
  },
  {
    id: 'telemetry-boundary',
    required: true,
    ownerAgent: 'boundary-risk-auditor',
    description: 'Telemetry events, diagnostics, traces, and error-reporting payloads are reviewed for sensitive data and release toggles.',
    examples: Object.freeze(['analytics events', 'diagnostic payloads', 'crash reporting config', 'opt-out behavior'])
  },
  {
    id: 'packaging-surface',
    required: true,
    ownerAgent: 'packaging-surface-auditor',
    description: 'Package metadata, artifact contents, bundled files, generated assets, version, changelog, and tags are ready.',
    examples: Object.freeze(['package files list', 'dist/build output', 'version bump', 'changelog entry', 'tag plan'])
  },
  {
    id: 'dependency-license-security',
    required: false,
    ownerAgent: 'packaging-surface-auditor',
    description: 'Dependency, lockfile, license, and vulnerability changes are reviewed when the release changes runtime or build dependencies.',
    examples: Object.freeze(['lockfile diff', 'audit result', 'license review', 'SBOM/provenance note'])
  },
  {
    id: 'release-automation',
    required: true,
    ownerAgent: 'packaging-surface-auditor',
    description: 'CI, tagging, publishing, deploy, and release-note automation is reviewed and has a safe dry-run or approval boundary.',
    examples: Object.freeze(['CI jobs', 'GitHub Actions', 'publish token scope', 'manual approval gate', 'dry-run log'])
  }
]);

const PHASES = Object.freeze([
  {
    id: 'validate-scope',
    title: 'Validate release scope',
    agent: 'release-scope-controller',
    objective: 'Confirm the branch/build target is explicit before any approval language is allowed.',
    requiredEvidence: Object.freeze(['target-scope']),
    outputs: Object.freeze(['normalized target', 'release type', 'scope assumptions', 'out-of-scope areas'])
  },
  {
    id: 'inspect-command-inventory',
    title: 'Inspect command inventory',
    agent: 'command-inventory-auditor',
    objective: 'Map all commands that can build, test, package, publish, migrate, tag, or deploy.',
    requiredEvidence: Object.freeze(['command-inventory', 'quality-signal']),
    outputs: Object.freeze(['command map', 'safe commands', 'dangerous commands', 'missing checks'])
  },
  {
    id: 'inspect-boundary-evidence',
    title: 'Inspect privacy, provider, storage, and telemetry evidence',
    agent: 'boundary-risk-auditor',
    objective: 'Review release-critical data and runtime boundaries before packaging is approved.',
    requiredEvidence: Object.freeze(['privacy-boundary', 'provider-routing', 'storage-and-recovery', 'telemetry-boundary']),
    outputs: Object.freeze(['boundary evidence', 'boundary unknowns', 'release blockers'])
  },
  {
    id: 'inspect-packaging-surface',
    title: 'Inspect packaging and release surfaces',
    agent: 'packaging-surface-auditor',
    objective: 'Verify artifact contents, versioning, changelog, dependencies, and release automation.',
    requiredEvidence: Object.freeze(['packaging-surface', 'release-automation']),
    outputs: Object.freeze(['artifact inventory', 'version/changelog status', 'automation risk', 'dependency notes'])
  },
  {
    id: 'classify-blockers',
    title: 'Identify blockers and evidence gaps',
    agent: 'release-gatekeeper',
    objective: 'Apply the release rubric and separate blockers from warnings, suggestions, and unknowns.',
    requiredEvidence: Object.freeze(['all required evidence groups or explicit missing-evidence records']),
    outputs: Object.freeze(['blockers', 'warnings', 'evidence gaps', 'conditional requirements'])
  },
  {
    id: 'return-verdict',
    title: 'Return release verdict',
    agent: 'release-gatekeeper',
    objective: 'Return a clear release decision with evidence, confidence, and the next safest action.',
    requiredEvidence: Object.freeze(['verdict rationale', 'safe next step']),
    outputs: Object.freeze(['readiness-verdict', 'evidence-present', 'evidence-missing', 'release-blockers', 'safe-next-step'])
  }
]);

const VERDICT_POLICY = Object.freeze({
  allowedVerdicts: Object.freeze(['needs-input', 'blocked', 'conditional', 'ready']),
  rules: Object.freeze([
    {
      verdict: 'needs-input',
      when: 'No branch, tag, package, app, release channel, or build target was provided.'
    },
    {
      verdict: 'blocked',
      when: 'Any required evidence group is missing, any release blocker is present, or privacy/provider/storage/telemetry status is unknown in strict mode.'
    },
    {
      verdict: 'conditional',
      when: 'No blocker is confirmed, but non-critical evidence gaps remain and each gap has a concrete pre-release check.'
    },
    {
      verdict: 'ready',
      when: 'All required evidence is present, no blockers remain, release automation is reviewed, and the next step is a dry-run or approved release action.'
    }
  ]),
  hardStops: Object.freeze([
    'Do not approve release readiness when the target scope is missing.',
    'Do not convert missing evidence into a suggestion.',
    'Do not mark privacy, provider, storage, telemetry, or release automation unknowns as ready.',
    'Do not recommend a publish/deploy/tag action before dry-run and rollback evidence are recorded.',
    'Do not merge blockers and suggestions into the same list.'
  ])
});

const OUTPUT_SCHEMA = Object.freeze({
  type: 'object',
  additionalProperties: false,
  required: [
    'readinessVerdict',
    'scope',
    'evidencePresent',
    'evidenceMissing',
    'releaseBlockers',
    'warnings',
    'safeNextStep'
  ],
  properties: {
    readinessVerdict: { type: 'string', enum: ['needs-input', 'blocked', 'conditional', 'ready'] },
    scope: { type: ['string', 'null'] },
    confidence: { type: 'string', enum: ['low', 'medium', 'high'] },
    evidencePresent: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'summary'],
        additionalProperties: false,
        properties: {
          id: { type: 'string' },
          summary: { type: 'string' },
          source: { type: 'string' }
        }
      }
    },
    evidenceMissing: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'whyItMatters', 'howToCollect'],
        additionalProperties: false,
        properties: {
          id: { type: 'string' },
          whyItMatters: { type: 'string' },
          howToCollect: { type: 'string' }
        }
      }
    },
    releaseBlockers: {
      type: 'array',
      items: {
        type: 'object',
        required: ['surface', 'severity', 'blocker', 'requiredFix'],
        additionalProperties: false,
        properties: {
          surface: { type: 'string' },
          severity: { type: 'string', enum: ['critical', 'high', 'medium'] },
          blocker: { type: 'string' },
          requiredFix: { type: 'string' }
        }
      }
    },
    warnings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['surface', 'warning', 'recommendedFollowUp'],
        additionalProperties: false,
        properties: {
          surface: { type: 'string' },
          warning: { type: 'string' },
          recommendedFollowUp: { type: 'string' }
        }
      }
    },
    safeNextStep: { type: 'string' }
  }
});

function normalizeText(value) {
  return String(value || '')
    .replace(/\s+/g, ' ')
    .trim();
}

function uniqueStrings(values) {
  return Array.from(new Set(values.filter(Boolean).map(normalizeText))).filter(Boolean);
}

function normalizeOptions(rawOptions) {
  const options = Object.assign({}, DEFAULT_OPTIONS, rawOptions || {});
  return {
    strict: options.strict !== false,
    includeAgentPrompts: options.includeAgentPrompts !== false,
    failOnUnknowns: options.failOnUnknowns !== false,
    maxEvidenceGapsForConditional: Number.isInteger(options.maxEvidenceGapsForConditional)
      ? Math.max(0, options.maxEvidenceGapsForConditional)
      : DEFAULT_OPTIONS.maxEvidenceGapsForConditional
  };
}

function buildAgentPrompts(scope) {
  const targetLine = scope
    ? `Release target: ${scope}`
    : 'Release target: MISSING. Stop and request a branch, tag, package, app, or build target.';

  return Object.freeze({
    'release-scope-controller': [
      targetLine,
      'Validate the release scope. Identify release type, intended artifact, release channel, and what is explicitly out of scope.',
      'Return target summary, assumptions, and any missing input. Do not approve readiness.'
    ].join('\n'),

    'command-inventory-auditor': [
      targetLine,
      'Find build, test, lint, typecheck, package, publish, deploy, tag, migration, and release-note commands.',
      'Classify each command as safe, read-only, dry-run, approval-required, or dangerous. Flag commands that can mutate external state.'
    ].join('\n'),

    'boundary-risk-auditor': [
      targetLine,
      'Review privacy, provider routing, storage/recovery, and telemetry evidence for this release.',
      'Treat missing evidence as a blocker in strict mode. Identify secret, PII, prompt, trace, region, fallback, migration, and rollback risks.'
    ].join('\n'),

    'packaging-surface-auditor': [
      targetLine,
      'Inspect package metadata, artifact contents, bundled files, versioning, changelog, dependencies, lockfiles, and release automation.',
      'Verify package allowlists/ignore files and dry-run publish/build evidence. Flag unknown artifact contents as blockers.'
    ].join('\n'),

    'release-gatekeeper': [
      targetLine,
      'Synthesize all audit results into the required output schema.',
      'Use the verdict policy: blockers and strict-mode unknowns force blocked; suggestions must never hide missing evidence.',
      'Return only a structured readiness result with evidence-present, evidence-missing, blockers, warnings, and safe-next-step.'
    ].join('\n')
  });
}

function buildContract(branchOrBuildTarget, rawOptions) {
  const scope = normalizeText(branchOrBuildTarget);
  const hasInput = Boolean(scope);
  const options = normalizeOptions(rawOptions);
  const requiredEvidenceIds = EVIDENCE_REQUIREMENTS.filter((item) => item.required).map((item) => item.id);

  const contract = {
    status: hasInput ? 'active-contract' : 'needs-input',
    workflow: WORKFLOW_NAME,
    version: WORKFLOW_VERSION,
    meta,
    intent: 'Check packaging and release readiness before a build, publish, deploy, or tagged release.',
    requiredInput: {
      name: 'branchOrBuildTarget',
      provided: hasInput,
      value: hasInput ? scope : null,
      examples: ['main', 'release/v1.8.0', 'tag:v1.8.0', 'app:web', 'package:@scope/name', 'build:production']
    },
    scope: hasInput ? scope : null,
    options,
    relatedCommands: RELATED_COMMANDS,
    orchestrationPattern: {
      primary: 'fan-out-and-synthesize',
      verification: 'adversarial-release-gate',
      rationale: 'Independent evidence collectors review command, boundary, and packaging surfaces, then a gatekeeper synthesizes a strict verdict.'
    },
    phaseOrder: PHASES.map((phase) => phase.title),
    phases: PHASES,
    agents: AGENTS,
    skills: uniqueStrings(AGENTS.flatMap((agent) => Array.from(agent.skills || []))),
    riskSurfaces: RISK_SURFACES,
    evidenceRequirements: EVIDENCE_REQUIREMENTS,
    requiredEvidenceIds,
    gatingChecks: [
      'target scope is explicit before review starts',
      'evidence is present before release approval',
      'privacy, provider, storage, telemetry, and automation unknowns block strict approval',
      'blockers are separated from warnings and suggestions',
      'release-impacting commands are classified before use',
      'artifact contents and publish scope are reviewed before release',
      'rollback or recovery path is known for storage or migration changes',
      'safe next step is dry-run, fix blockers, collect evidence, or approved release action only'
    ],
    verdictPolicy: VERDICT_POLICY,
    outputSections: [
      'readiness-verdict',
      'scope',
      'evidence-present',
      'evidence-missing',
      'release-blockers',
      'warnings',
      'safe-next-step'
    ],
    outputSchema: OUTPUT_SCHEMA,
    safeDefaults: {
      initialVerdict: hasInput ? 'blocked-until-evidence-is-collected' : 'needs-input',
      approvalBias: 'fail-closed',
      missingEvidenceTreatment: options.failOnUnknowns ? 'blocker' : 'conditional-gap',
      commandExecutionPolicy: 'discover commands first; prefer dry-run; require approval for publish/deploy/tag/migration commands'
    },
    safeNextStep: hasInput
      ? 'Collect the required evidence groups, run non-destructive checks first, then synthesize a blocked/conditional/ready verdict using the output schema.'
      : 'Provide a branch, tag, package, app, release channel, or build target before running the release readiness gate.'
  };

  if (options.includeAgentPrompts) {
    contract.agentPrompts = buildAgentPrompts(scope);
  }

  return contract;
}

function emitContract(contract, stream) {
  const output = `${JSON.stringify(contract, null, 2)}\n`;
  const destination = stream || process.stdout;
  destination.write(output);
}

function parseCliArgs(argv) {
  const options = {};
  const positional = [];

  for (const arg of argv || []) {
    if (arg === '--no-prompts') {
      options.includeAgentPrompts = false;
    } else if (arg === '--relaxed') {
      options.strict = false;
      options.failOnUnknowns = false;
    } else if (arg.startsWith('--max-conditional-gaps=')) {
      const rawValue = arg.slice('--max-conditional-gaps='.length);
      const parsed = Number.parseInt(rawValue, 10);
      if (Number.isInteger(parsed)) {
        options.maxEvidenceGapsForConditional = parsed;
      }
    } else if (arg.startsWith('--target=')) {
      positional.push(arg.slice('--target='.length));
    } else if (arg.startsWith('--')) {
      // Preserve forward compatibility: unknown flags are ignored rather than treated as release scope.
    } else {
      positional.push(arg);
    }
  }

  return {
    scope: normalizeText(positional.join(' ')),
    options
  };
}

if (require.main === module) {
  const cli = parseCliArgs(process.argv.slice(2));
  emitContract(buildContract(cli.scope, cli.options));
}

module.exports = {
  meta,
  buildContract,
  buildAgentPrompts,
  parseCliArgs,
  emitContract
};