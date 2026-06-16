# Operational README Blueprint

Use this blueprint to create README files that function as an operating manual plus a trust contract. Adapt the order only when the repository type clearly benefits from a different flow.

## Evidence inventory to extract first

Capture these facts before writing:

| Field | Grounding source examples | Output rule |
|---|---|---|
| Project name | package manifest, module manifest, root directory, existing README | Use the strongest repo-local name. |
| Repo type | source layout, entry points, manifests, CI | Mark `[INFERRED]` if heuristic. |
| Primary language | lockfiles, manifests, file counts | Prefer config over file count. |
| Framework/runtime | package deps, pyproject, Cargo.toml, go.mod, Dockerfile | Include version when present. |
| Package manager | lockfiles and packageManager fields | Do not guess when multiple managers conflict. |
| Entry points | package scripts, src/main, cli files, app routes, Docker CMD | Use real file paths. |
| Install command | lockfiles, manifest, existing docs | Mark `[INFERRED]` if convention-only. |
| Run/dev command | scripts, Makefile, CLI entry point | Use exact script names. |
| Build command | scripts, Makefile, CI | Omit if absent. |
| Test command | scripts, Makefile, CI, test config | State honestly if none found. |
| Lint/typecheck | scripts, config files, CI | Separate lint and typecheck when possible. |
| Deployment target | Dockerfile, compose, CI, cloud config | Do not invent hosting. |
| License | LICENSE file or manifest license field | If absent, say no license file was found. |
| CI provider | `.github/workflows`, GitLab, Azure, CircleCI files | Link only existing workflow paths. |
| Major directories | top-level project dirs | Include important dirs only. |
| Public API/CLI/commands | source exports, CLI configs, scripts, docs | Prefer documented public surfaces. |
| Environment variables | `.env.example`, `.env.sample`, docs | Never read `.env` values. |
| Contribution files | CONTRIBUTING, PR templates, issue templates | Link only existing files. |
| Security/governance files | SECURITY, CODE_OF_CONDUCT, license | Mark missing policies as `[TBD]`. |

## Default README structure

1. Title, badges, and one-sentence value proposition
2. Three high-signal benefit bullets
3. Table of Contents
4. Quickstart
5. Features
6. Architecture
7. Directory Structure
8. Usage
9. Configuration
10. Developer Command Center
11. Testing & Verification
12. Troubleshooting
13. Stack Inventory
14. Reproducibility & Maintenance
15. Contributing
16. Governance
17. Roadmap
18. License

## Section rules

### Project header

- Use the discovered project name as the H1.
- Add only badges that point to real local or remote evidence.
- The opening sentence must state what the project is, who it helps, and why it matters.
- Avoid vague claims such as “powerful,” “modern,” “robust,” “next-generation,” or “AI-powered” unless the repository proves the claim.

### Quickstart

Include these subsections when applicable:

- Prerequisites
- Install
- Configure
- Run
- Verify

Rules:

- Provide copy/paste commands.
- Include expected output when known from scripts, docs, or observed safe command output.
- When configuration exists, reference `.env.example`, `.env.sample`, or documented config files.
- Do not write “configure as needed” without naming required variables or config paths.

### Features

- Use concrete capabilities grounded in files, APIs, scripts, or docs.
- Each bullet should explain the capability and why it matters.
- Remove marketing-only features that cannot be tied to repository evidence.

### Architecture

Use a compact Mermaid diagram when it helps the reader understand the system. For applications, show user/client, frontend or CLI, backend/core runtime, storage, and external APIs only when those components exist. For libraries, show package/module flow. Explain every node in plain English.

### Directory structure

Show important directories only. Add a short comment explaining why each path matters. Do not dump every file.

### Usage

Choose the repo-appropriate shape:

| Repo type | Usage content |
|---|---|
| Application | Main run command and common workflows table. |
| CLI | Install command, command reference, examples, options, exit behavior if documented. |
| Library/package | Minimal import/use example, API surface, compatibility notes. |
| Backend/API | Local server command, auth, endpoint overview, API docs path, database setup. |
| AI agent/automation | Command center, agents, skills/workflows, hooks, permissions, verification gates, safety model. |
| Data/ML | Dataset source, training/evaluation commands, metrics, hardware assumptions. |
| Smart contract/Web3 | Chain support, toolchain, deployment safety, ABI generation, audit status. |

### Configuration

Use a table with variable, required status, default, source, and description. Use safe example files only. Never expose actual secrets. If no config is found, state that no required environment variables were discovered.

### Developer Command Center

Use this section when the repo has scripts, Make targets, CLIs, agents, workflows, hooks, or automation. Include command, category, when to use, source, and purpose.

For AI-agent repositories, also include these tables when evidence exists:

| Table | Required columns |
|---|---|
| Agents | Agent, role, scope, source path |
| Skills / Workflows | Skill or workflow, when to invoke, what it checks, source path |
| Hooks / Permissions | Hook or permission, event/scope, purpose, source path |

### Testing & Verification

- Include actual test, lint, typecheck, and build commands when present.
- State honestly when no tests or no CI are found.
- Recommend the shortest grounded verification path before opening a PR.
- Do not claim commands pass unless fresh output has been observed.

### Troubleshooting

Include at least three likely issues. Prefer repo-specific symptoms from package manager, runtime, environment variables, ports, database setup, auth, build tools, or missing external services. Each row must include symptom, likely cause, and exact fix.

### Stack Inventory

Use real versions from manifests and lockfiles when possible:

| Layer | Technology | Version | Source | Notes |
|---|---|---|---|---|

Use `Unknown` when no version is declared. Use `[INFERRED]` only when a claim is strongly implied but not explicitly declared.

### Reproducibility & Maintenance

Include commands or documented procedures for:

- Fresh clone verification
- Dependency updates
- Resetting local state
- Platform notes for Windows, WSL, macOS, Docker, or Linux when relevant

Do not invent maintenance policy. If missing, recommend files or scripts to add.

### Contributing

If contribution files exist, link to them. If not, include a short honest note: “Contribution guidelines are not yet formalized. Please open an issue before large changes.”

### Governance

Use a table for Code of Conduct, Security, License, Maintainers, and Support. Link only to files that exist. Use `[TBD]` for missing governance areas.

### Roadmap

Keep it short. Use existing issues, project docs, TODO comments, release notes, or user input. Do not invent commitments.

### License

If a license file exists, name it and link it. If no license file exists, say: “No license file was found. Add a license before publishing or accepting contributions.”

## Repo-type adaptation priorities

| Repo type | Prioritize |
|---|---|
| Web app | Quickstart, env vars, screenshots if real, architecture, routes/API, deployment, troubleshooting. |
| CLI tool | Install, command reference, examples, exit codes if documented, config, shell completion if present. |
| Library/package | Installation, minimal usage, API reference, version compatibility, examples, testing. |
| AI agent / Claude Code / automation | Command center, agents, skills, hooks, rules, permissions, verification gates, safety model. |
| Backend/API | API endpoints, auth, database setup, migrations, local dev, OpenAPI/Swagger, deployment. |
| Data/ML | Dataset source, model architecture, training, evaluation, reproducibility, hardware assumptions, metrics. |
| Smart contract/Web3 | Chain support, contract addresses only if present, toolchain, deployment safety, ABI generation, wallet flows, audit status. |

## Quality rubric

Score 0–3 for each category. Target total is 24 or higher out of 30.

| Category | 0 | 1 | 2 | 3 |
|---|---|---|---|---|
| Positioning | Unclear | Says what it is | Says who it helps | Says what, who, and why |
| Quickstart | Missing | Partial | Works with assumptions | Fully copy/paste runnable |
| Accuracy | Invented | Mostly inferred | Mostly grounded | Fully grounded in repo |
| Structure | Random | Basic sections | Clear flow | Scannable and complete |
| Commands | Missing | Generic | Mostly real | Real and verified or clearly sourced |
| Architecture | Missing | Vague | Useful | Diagram plus component roles |
| Troubleshooting | Missing | Generic | Some repo-specific | Practical matrix |
| Maintenance | Missing | Minimal | Some commands | Reproducible workflows |
| Governance | Missing | License only | Some links | Contribution, security, license, support status |
| Adaptation | Generic | Slightly tailored | Repo-aware | Strongly repo-type specific |

## Final self-check

Before completing, verify:

- A new user can identify the project in one sentence.
- They can install and run it.
- They can verify it works or see that no verification exists.
- They can find the main commands.
- They can understand the architecture or module flow.
- They can troubleshoot likely failures.
- They can identify the stack and version sources.
- They can contribute safely.
- All claims are grounded, inferred, or explicitly marked missing.
- No placeholder tokens, invented links, invented badges, or fabricated commands remain.
