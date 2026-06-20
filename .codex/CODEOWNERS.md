# Agent routing map

Not GitHub CODEOWNERS — GitHub can't assign reviews to local sub-agents (no GitHub identity). This is an informal lookup for which `.claude/agents/` agent to invoke when working on a given path. Last match wins, same convention as real CODEOWNERS.

| Path | Agent | Why |
|---|---|---|
| `*` | lead-engineer | default orchestrator, decomposes and routes to the others |
| `src-tauri/src/security/`, `security/` | silas-mercer | AppSec/threat-modeling lead |
| `src-tauri/src/ipc/`, `src-tauri/src/providers/` | kaelen-vance | structural/contract boundaries between layers |
| `src-tauri/src/storage/` | kaelen-vance | lifecycle/resource versioning is their core axiom |
| `src/`, `src-tauri/src/app_state.rs` | kaelen-vance | architecture & simplicity |
| `scripts/`, CI workflow failures | aris-thorne | forensic debugging, root-cause tracing |
| test suites, lint/build gates | jax-holden | runs and verifies, doesn't trust claims |
| multi-agent/orchestration docs (`AGENTS.md`, `plans/`) | elara-voss | A2A contracts between agents |
| stale `git worktree` cleanup | worktree-maintainer | only job it has |

Real GitHub review/merge gating still comes from `.github/CODEOWNERS` (`@JackSmack1971`).
