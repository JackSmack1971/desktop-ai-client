# Desktop AI Client

Monorepo for the desktop client shell, the Tauri backend, and supporting docs and tests.

## Intent Layer

Read the nearest `AGENTS.md` before editing code in a subdirectory.

- `src-tauri/AGENTS.md` - Rust/Tauri crate boundary, build config, migrations
- `src-tauri/src/AGENTS.md` - shared backend module tree and `AppState`
- `src-tauri/src/ipc/AGENTS.md` - frontend-facing commands
- `src-tauri/src/providers/AGENTS.md` - provider routing, OpenRouter adapter, SSE parsing
- `src-tauri/src/security/AGENTS.md` - secrets, file tokens, artifact sandboxing, command policy
- `src-tauri/src/storage/AGENTS.md` - SQLite pool, migrations, typed stores
- `src-tauri/src/telemetry/AGENTS.md` - audit log and release evidence
- `src/lib/stores/AGENTS.md` - frontend reactive stores (surface, chat, artifacts, history, settings)

## Global Invariants

- Every IPC command needs three coupled registrations: `tauri::generate_handler!` in `src-tauri/src/main.rs`, a capability entry in `src-tauri/capabilities/main.json`, and a row in `security/command-inventory.toml`. `src-tauri/src/bin/verify-command-inventory.rs` cross-checks all three.
- Secrets are backend-owned: never hold one across an `.await`, never let it reach a log macro, error string, or IPC response. Wrap in-memory credentials in `secrecy::SecretString`. There is currently **no redaction module** — `security::redaction` was deleted as a dead stub (commit `c7fffd1`) — so "redact before logging" today means "don't log the field at all," not a redaction layer catching it after the fact.
- File paths never cross IPC as raw strings — only opaque tokens minted by `security::file_tokens`.
- The chat system prompt, conversation titles, and provider-key resolution are backend-generated/resolved and never accepted as IPC input from the frontend.
- Keep backend-owned concerns backend-owned: command policy, provider routing, storage, and telemetry stay out of the renderer.
- `docs/architecture.md` describes an unrelated agent system (Planner/Executor/Memory/Judge) and does not describe this codebase; `docs/threat-model.md` and `docs/privacy-boundaries.md` are stub focus-area lists. Prefer `.planning/codebase/ARCHITECTURE.md` for current-state architecture claims — but not `.planning/codebase/CONCERNS.md`/`CONVENTIONS.md`/`TESTING.md`, which are dated 2026-06-13 and describe a pre-implementation scaffold the v1.0 milestone has since superseded.
- Prefer small, local AGENTS nodes when a subsystem has distinct ownership or invariants.

## Four Agent Rules

**1. Think Before Coding**  
**Don't assume. Don't hide confusion. Surface tradeoffs.**

Before implementing:
- State your assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them — don't pick silently.
- If a simpler approach exists, say so. Push back when warranted.
- If something is unclear, stop. Name what's confusing. Ask.

**2. Simplicity First**  
**Minimum code that solves the problem. Nothing speculative.**

- No features beyond what was asked.
- No abstractions for single-use code.
- No "flexibility" or "configurability" that wasn't requested.
- No error handling for impossible scenarios.
- If you write 200 lines and it could be 50, rewrite it.

Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

**3. Surgical Changes**  
**Touch only what you must. Clean up only your own mess.**

When editing existing code:
- Don't "improve" adjacent code, comments, or formatting.
- Don't refactor things that aren't broken.
- Match existing style, even if you'd do it differently.
- If you notice unrelated dead code, mention it — don't delete it.

When your changes create orphans:
- Remove imports/variables/functions that *your* changes made unused.
- Don't remove pre-existing dead code unless asked.

The test: Every changed line should trace directly to the user's request.

**4. Goal-Driven Execution**  
**Define success criteria. Loop until verified.**

Transform tasks into verifiable goals:
- "Add validation" → "Write tests for invalid inputs, then make them pass"
- "Fix the bug" → "Write a test that reproduces it, then make it pass"
- "Refactor X" → "Ensure tests pass before and after"

For multi-step tasks, state a brief plan:
```
1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]
```

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## Working Rules

- Prefer the smallest correct change, update docs when behavior or boundaries change, and verify with the narrowest meaningful command set.

==================================

Universal Agent Constitution
============================

    ENGINEERING CONSTITUTION


    1. Repository evidence outranks prior assumptions.
    2. Explicit requirements outrank inferred preferences.
    3. External verification outranks self-assessment.
    4. Public contracts must not change accidentally.
    5. Security boundaries must not be weakened for convenience.
    6. Tests must not be altered merely to conceal a defect.
    7. Existing unrelated user changes must be preserved.
    8. Scope expansion requires explicit justification.
    9. Uncertainty must be reported rather than disguised.
    10. Completion requires traceable evidence.
    11. Failed approaches must not be repeated without new evidence.
    12. Agents may recommend, but tools and artifacts establish facts.

## Git Worktree Hygiene (MANDATORY)

Before starting any new task or after completing work that used a worktree:
1. Run `git worktree list` to inventory all worktrees.
2. Identify stale ones: worktrees whose branch no longer exists OR whose directory is gone/missing its .git file.
3. Prune administrative state: `git worktree prune`
4. Remove actual stale worktree directories with `git worktree remove <path>` (use --force only if confirmed dead).
5. Verify with `git worktree list` again — only active worktrees should remain.

**Rules:**
- Surgical: Only touch worktrees you created or that are verifiably stale. Never delete active user worktrees.
- Goal-driven: Success = clean `git worktree list` output with zero stale entries.
- Think first: If unsure whether a worktree is safe to remove, list it and ask before deleting.
- Simplicity: Prefer `git worktree prune` + targeted `remove` over manual rm -rf.

Never leave the repo with orphaned worktree directories. Treat worktree cleanup as a required post-task verification step.
