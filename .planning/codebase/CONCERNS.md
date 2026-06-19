# Codebase Concerns

**Analysis Date:** 2026-06-18

---

## Implementation Status

The previous version of this document (dated 2026-06-13) described the
`ipc`, `providers`, `security`, `storage`, and `telemetry` layers as "all
stub functions." That is no longer accurate. As of this audit:

- **IPC layer — 7 of 8 modules implemented, 15 commands registered:** `app_shell` (113 lines), `chat` (723 lines — streaming, cancellation, storage wiring, artifact detection), `history` (236 lines), `artifacts` (100 lines), `privacy` (115 lines), `files` (155 lines), and `inventory` (726 lines — the cross-source command verifier) all contain real implementations. Only `ipc::providers.rs` remains a 1-line stub, and it is not registered in `generate_handler!`.
- **Providers layer — 3 of 4 modules implemented:** `routing` (105 lines), `openrouter` (138 lines), and `sse` (278 lines) are fully implemented. `capabilities.rs` remains a 1-line stub — capability detection is currently implicit in `routing` rather than a separate module.
- **Security layer — fully implemented:** `secrets.rs` (352 lines, OS-keychain-backed via the `keyring` crate), `file_tokens.rs` (82 lines), `artifact_sandbox.rs` (314 lines), `command_policy.rs` (91 lines, now the single authority used by every IPC module), `redaction.rs` (46 lines).
- **Telemetry layer — fully implemented:** `audit_log.rs` (64 lines), `release_evidence.rs` (394 lines, wired to the same inventory verifier used by `verify-command-inventory`).
- **Storage layer — 4 of 5 modules implemented:** `sqlite.rs` (475 lines), `migrations.rs` (411 lines, 4 applied migrations), `fts.rs` (169 lines), `retention.rs` (142 lines), plus `artifacts.rs` (294 lines, not present in the prior audit's module list). `backup.rs` remains a 7-line stub. `src-tauri/migrations/` still contains only `.gitkeep` — migrations are still embedded as `&'static str` constants in Rust rather than discrete `.sql` files; this is a known, accepted tradeoff, not an oversight (see Architectural Drift below).
- **Frontend surfaces — implemented, not placeholders:** `ChatSurface.svelte`, `HistorySurface.svelte`, and `ArtifactsSurface.svelte` render real content backed by `src/lib/stores/{chat,history,artifacts}.ts` and component groups under `src/lib/components/{chat,history}/`. `SettingsSurface.svelte` is backed by `src/lib/stores/settings.ts` for credential status display.

**Remaining genuine gaps:**
- `ipc::providers.rs` — 1-line stub, unregistered, no tracked requirement currently points at it (see Dead Code below).
- `providers::capabilities.rs` — 1-line stub; capability detection logic lives inline in `routing.rs` instead.
- `storage::backup.rs` — 7-line stub; no backup/restore implementation exists yet.
- `docs/threat-model.md` and `docs/privacy-boundaries.md` — both still 12-line stub files with only a header and a focus-area list; no actual threat analysis or boundary definitions.

---

## Security Concerns

**Command policy is now the single enforcement authority:**
- Files: `src-tauri/src/security/command_policy.rs`, `src-tauri/src/ipc/{app_shell,chat,history}.rs`
- Prior state: `app_shell`, `chat`, and `history` each enforced window-label checks via a private, duplicated `assert_main_window` helper, bypassing `command_policy::policy_check` entirely — tracked and fixed as issue #2.
- Current state: all IPC modules call `command_policy::policy_check(command, window_label)`, which validates both window label and command-name membership against a single static table. `ipc::inventory::verify_inventory()` now reconciles that table against `security/command-inventory.toml`, registered handlers, permission files, capability files, and release capabilities — six sources, cross-checked by `cargo run --bin verify-command-inventory` and by `collect-release-evidence`.

**`docs/threat-model.md` and `docs/privacy-boundaries.md` remain stubs:**
- Files: `docs/threat-model.md`, `docs/privacy-boundaries.md`
- Risk: Both files contain only a header and a focus-area list with no actual content, despite the security layer (secrets, file tokens, command policy, redaction, artifact sandbox) being fully implemented. There is no written spec to validate the implementation against.
- Fix approach: Populate both with the boundaries actually enforced in code today (window-label + command-name policy, opaque file tokens, keychain-backed secrets, redaction-before-telemetry).

**`AppShell.svelte` is dead code:**
- Files: `src/lib/components/AppShell.svelte`
- Status: Confirmed via `grep -rn "from '\$lib/components/AppShell'" src/` — no remaining imports anywhere in `src/`. `src/routes/+page.svelte` imports and renders `WorkspaceShell.svelte` instead. The double-rail risk this file posed (CR-01, documented in `01-REVIEW.md`) is currently dormant since nothing imports it, but the file itself was never deleted.
- Fix approach: Delete `src/lib/components/AppShell.svelte`.

---

## Technical Debt

**`normalizeIpcError` duplicated across 5 frontend stores:**
- Files: `src/lib/stores/chat.ts`, `surface.ts`, `history.ts`, `artifacts.ts`, `settings.ts`
- Issue: Each store defines its own copy of `normalizeIpcError`. The prior audit only knew about the `surface.ts` copy because `chat.ts`/`history.ts`/`artifacts.ts`/`settings.ts` did not exist yet; the duplication has grown with each new store added (tracked as issue #6).
- Fix approach: Extract one shared implementation (e.g. `src/lib/api/errors.ts`) and have all five stores import it.

**Migrations are embedded Rust constants, not `.sql` files:**
- Files: `src-tauri/src/storage/migrations.rs`, `src-tauri/migrations/.gitkeep`
- Issue: All 4 applied migrations live as `&'static str` constants in `migrations.rs`. The `src-tauri/migrations/` directory still exists but holds only `.gitkeep`. This was flagged as an open question in the prior audit ("Phase 3 will need a decision"); History (Phase 3) has since shipped without resolving it, so the ambiguity persists for any future migration author.
- Fix approach: Either commit to the embedded-constant pattern explicitly in `backend.md`/`AGENTS.md` and remove the empty directory, or migrate to discrete `.sql` files per the storage rules in `.claude/rules/backend.md`.

**`ipc::providers.rs` is an unregistered, unreferenced stub:**
- Files: `src-tauri/src/ipc/providers.rs` (1 line), `src-tauri/src/ipc/mod.rs:20`
- Issue: Declared as a `pub mod` but contains no command and is never registered in `generate_handler!`. No requirement in `.planning/REQUIREMENTS.md` or `.planning/ROADMAP.md` currently names a "providers status" IPC surface (tracked as issue #5).
- Fix approach: Delete the module and its `mod.rs` declaration if no requirement supersedes it, or replace the stub with a tracked TODO referencing the specific requirement ID if one is added.

**`storage::backup.rs` and `providers::capabilities.rs` remain stubs:**
- Files: `src-tauri/src/storage/backup.rs` (7 lines), `src-tauri/src/providers/capabilities.rs` (1 line)
- Issue: Neither has a real implementation. `backup.rs` has no backup/restore behavior; `capabilities.rs` has no standalone capability-detection logic (it lives inline in `providers::routing` instead).
- Priority: Low unless a roadmap phase specifically requires standalone backup or capability-detection APIs — confirm against `.planning/ROADMAP.md` before prioritizing.

---

## Build / Runtime Gaps

**No CI pipeline confirmed:**
- Files: `.github/workflows/.gitkeep` (directory present, no workflow files)
- Risk: No automated build, test, or type-check runs on push. All verification evidence in phase plans and PRs has been produced manually, not by CI.

**Building `src-tauri` requires a Linux GTK/WebKit toolchain not available in every environment:**
- Risk: `cargo check`/`cargo test` for the `src-tauri` crate requires `gdk-3.0`/`webkit2gtk-4.1` system libraries via pkg-config. Environments without these libraries (and without unrestricted package-manager network access) cannot compile or test the Rust side at all — this has affected verification rigor on at least one recent PR (#2, #7) and should be considered a known constraint, not a one-off failure.
- Mitigation in the interim: manual diff review and targeted `grep` verification, with the gap stated explicitly in PR descriptions per `.claude/rules/testing.md`.

**No frontend unit tests:**
- Files: `src/lib/stores/*.ts`, `src/lib/components/**/*.svelte`
- What's not tested: `find src -iname "*.test.ts" -o -iname "*.spec.ts"` returns no results. None of the five stores (`chat`, `history`, `surface`, `artifacts`, `settings`) have a test suite despite all being fully implemented.
- Risk: Store regressions (including the `normalizeIpcError` duplication above) surface only as runtime failures.

---

## Architectural Drift

**Migrations directory vs. embedded constants — open since Phase 1, still open after Phase 3:**
- `src-tauri/src/ipc/mod.rs`'s doc comment and the storage rules in `.claude/rules/backend.md` describe migrations as ordered, immutable files, but the actual implementation embeds them as Rust string constants. `src-tauri/migrations/` exists only as an empty placeholder. This was identified as ambiguous before Phase 3 (History) shipped and remains ambiguous now that Phase 3 is mostly complete — see Technical Debt above.

**`.planning/codebase/ARCHITECTURE.md` and this file previously understated implemented scope:**
- Both files were dated 2026-06-13 and described `ipc`, `providers`, `security`, `storage`, and `telemetry` as stub-only. By 2026-06-18, `main.rs` registers 15 commands, `chat.rs` is a 723-line full implementation, and `inventory.rs` is a 726-line real verifier. This drift is the subject of this regeneration (issue #4) and should not recur if these docs are refreshed alongside future roadmap phase completions rather than left until an audit catches the gap.

---

## Test Coverage Gaps

**Rust integration and unit test status:**
- Files: `src-tauri/tests/app_shell.rs`, `src-tauri/src/**/*.rs` (`#[cfg(test)] mod tests` blocks throughout)
- Status: Inline unit tests exist throughout the implemented modules (e.g. `command_policy.rs`, `app_shell.rs`, `chat.rs`, `history.rs`, `inventory.rs` all have `#[cfg(test)]` coverage for their success and error paths). `src-tauri/tests/app_shell.rs` provides integration-level coverage. Whether `cargo test --manifest-path src-tauri/Cargo.toml` passes in a properly provisioned environment (with GTK/WebKit system libraries) has not been confirmed by this audit — see Build/Runtime Gaps above.

**No frontend unit tests:**
- See Build/Runtime Gaps above.

**E2E, security, and adversarial fixture directories hold reference fixtures but no automated runner:**
- Files: `tests/fixtures/{provider-drift,capability-drift,adversarial-sse,hostile-renderer,srcdoc-escaping,sqlite-corruption,fts-query-abuse,wal-recovery}/`, `tests/e2e/release-evidence.md`, `tests/security/release-gate.md`
- What's there now: concrete fixture files exist (e.g. `openrouter.json`, `error-stream.txt`, `payload.html`, `recovery.sql`, `query.txt`) — these are real adversarial inputs, not just `.gitkeep` placeholders as in the prior audit.
- What's still missing: no automated test runner that consumes these fixtures was found; `tests/e2e/release-evidence.md` and `tests/security/release-gate.md` read as evidence/checklist documents rather than executable suites.
- Priority: Confirm against `.planning/ROADMAP.md` Phase 6 (Release) requirements whether fixture consumption is meant to be automated or remains a manual release-gate checklist by design.

---

*Concerns audit: 2026-06-18*
