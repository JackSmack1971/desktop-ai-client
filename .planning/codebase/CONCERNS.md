# Codebase Concerns

**Analysis Date:** 2026-06-13

---

## Missing Implementation (Scaffold Placeholders)

The following modules exist as declared `pub mod` entries in `lib.rs` and are referenced in the docs/roadmap, but every file contains only a single no-op stub function — none of these modules provide real behavior.

**IPC layer — 5 of 6 handlers are empty stubs:**
- `src-tauri/src/ipc/chat.rs` — `pub fn chat() { // Scaffold placeholder. }`
- `src-tauri/src/ipc/history.rs` — `pub fn history() { // Scaffold placeholder. }`
- `src-tauri/src/ipc/files.rs` — `pub fn files() { // Scaffold placeholder. }`
- `src-tauri/src/ipc/providers.rs` — `pub fn providers() { // Scaffold placeholder. }`
- `src-tauri/src/ipc/privacy.rs` — `pub fn privacy() { // Scaffold placeholder. }`
- `src-tauri/src/ipc/inventory.rs` — `pub fn inventory() { // Scaffold placeholder. }`

None of these stub functions are `#[tauri::command]`-annotated. Only `ipc::app_shell::get_active_surface` and `ipc::app_shell::set_active_surface` are registered in `main.rs`'s `invoke_handler`. All Phase 2–6 IPC surfaces are completely unimplemented.

**Providers layer — all 4 provider modules are stubs:**
- `src-tauri/src/providers/routing.rs` — `pub fn routing() { // Scaffold placeholder. }`
- `src-tauri/src/providers/openrouter.rs` — `pub fn openrouter() { // Scaffold placeholder. }`
- `src-tauri/src/providers/sse.rs` — `pub fn sse() { // Scaffold placeholder. }`
- `src-tauri/src/providers/capabilities.rs` — no real impl

Phase 2 (ROUTE-01, ROUTE-02) cannot begin until these are implemented.

**Security layer — all 5 security modules are stubs:**
- `src-tauri/src/security/secrets.rs` — `pub fn secrets() { // Scaffold placeholder. }`
- `src-tauri/src/security/file_tokens.rs` — `pub fn file_tokens() { // Scaffold placeholder. }`
- `src-tauri/src/security/artifact_sandbox.rs` — `pub fn artifact_sandbox() { // Scaffold placeholder. }`
- `src-tauri/src/security/command_policy.rs` — `pub fn command_policy() { // Scaffold placeholder. }`
- `src-tauri/src/security/redaction.rs` — `pub fn redaction() { // Scaffold placeholder. }`

Requirements SEC-01, SEC-02, SEC-03 are entirely unimplemented. The security layer referenced in `docs/privacy-boundaries.md` and `docs/threat-model.md` has no executable behavior.

**Telemetry layer — both modules are stubs:**
- `src-tauri/src/telemetry/audit_log.rs` — `pub fn audit_log() { // Scaffold placeholder. }`
- `src-tauri/src/telemetry/release_evidence.rs` — `pub fn release_evidence() { // Scaffold placeholder. }`

**Storage layer — 3 of 5 storage modules are stubs:**
- `src-tauri/src/storage/fts.rs` — `pub fn fts() { // Scaffold placeholder. }`
- `src-tauri/src/storage/backup.rs` — `pub fn backup() { // Scaffold placeholder. }`
- `src-tauri/src/storage/retention.rs` — `pub fn retention() { // Scaffold placeholder. }`

Requirements HIST-02 (search) and HIST-03 (retention) depend on these. `src-tauri/migrations/` contains only `.gitkeep` — all migrations live as static strings in Rust, not as discrete migration files.

**Frontend surfaces — all 4 content surfaces are placeholder stubs:**
- `src/lib/components/surfaces/ChatSurface.svelte` — renders placeholder text
- `src/lib/components/surfaces/HistorySurface.svelte` — renders placeholder text
- `src/lib/components/surfaces/SettingsSurface.svelte` — renders placeholder text
- `src/lib/components/surfaces/ArtifactsSurface.svelte` — renders placeholder text

These are intentional scaffolds documented in phase plans, but they block all UI-facing Phase 2–5 work.

---

## Security Concerns

**SEC-01 not enforced — no backend secret store:**
- Files: `src-tauri/src/security/secrets.rs`
- Risk: Provider credentials for routing (OpenRouter API keys, etc.) have no backend store, vault integration, or runtime protection. The stub function does nothing. Any Phase 2 work that sends prompts to an external provider without this will handle secrets in an undefined way.
- Current mitigation: None. No provider routing code exists yet, so no secrets are in play — but this must be built before Phase 2 ships.

**SEC-02 not enforced — no opaque file token system:**
- Files: `src-tauri/src/security/file_tokens.rs`
- Risk: No Rust-owned file selection or tokenization layer exists. If a Phase 2+ feature accepts file paths from the frontend before this is implemented, it would grant raw path authority to the renderer — an explicit out-of-scope violation per `REQUIREMENTS.md`.
- Current mitigation: No file intake IPC exists yet.

**SEC-03 not enforced — no redaction pipeline:**
- Files: `src-tauri/src/security/redaction.rs`, `src-tauri/src/telemetry/audit_log.rs`
- Risk: Sensitive content (prompts, paths, keys) has no redaction path before logging or telemetry. Any Phase 2 logging that emits IPC payloads will log raw content.
- Current mitigation: No telemetry or audit log runs yet.

**No command inventory cross-check:**
- Files: `src-tauri/src/ipc/inventory.rs` (stub), `src-tauri/capabilities/main.json`
- Risk: `src-tauri/src/ipc/mod.rs` doc comment states "The full list of registered commands must remain in sync with security/command-inventory.toml." No such file exists in the repository. Only 2 commands are registered today; as Phase 2 adds more, the inventory check has no enforcement mechanism until `command_policy.rs` and `inventory.rs` are implemented.
- Impact: REL-01 cannot be satisfied without this.

**`AppShell.svelte` is a dead component that re-renders `SurfaceRail`:**
- Files: `src/lib/components/AppShell.svelte`
- Risk: `AppShell.svelte` is no longer imported by any route after the CR-01 fix, but the file still exists and still imports and renders `<SurfaceRail />`. A future contributor who imports `AppShell` in any route will reintroduce the double-rail DOM bug (CR-01 from the Phase 01 review). Documented as IN-01 in `01-REVIEW.md` but not yet deleted.
- Fix approach: Delete `src/lib/components/AppShell.svelte`. It is superseded by `WorkspaceShell.svelte` and has no callers.

---

## Technical Debt

**`unwrap()` in production code path:**
- Files: `src-tauri/src/app_state.rs:94`
- Issue: `serde_json::to_string(&Surface::Chat).unwrap()` is in the `Default` impl for `AppState`. If `Surface` ever becomes non-serializable (e.g., a serde attribute typo), this panics the application at startup with no diagnostic.
- Fix approach: Replace with `.expect("Surface::Chat must be JSON-serializable")` to give a clear message, or use a string literal default.

**`SURFACE_LABELS` duplicated across 3 frontend files:**
- Files: `src/lib/stores/surface.ts:18-23`, `src/lib/components/WorkspaceShell.svelte:33-38`, `src/routes/+page.svelte:10-15`
- Issue: The `Surface → string` label mapping is defined identically in three places. Adding a new surface requires updating all three. Documented as IN-02 in `01-REVIEW.md` but not yet fixed.
- Fix approach: Export `SURFACE_LABELS` from `surface.ts` and import in the other two files.

**Migration savepoint uses `format!` string interpolation:**
- Files: `src-tauri/src/storage/migrations.rs:87-93`
- Issue: A `debug_assert!` was added (CR-04 fix) validating migration IDs are alphanumeric/underscore, but the assertion fires only in debug builds. The structural SQL interpolation pattern is documented with a `SAFETY` comment but remains a copy-paste risk for future contributors adding migrations. Release builds have no protection if the assertion is absent or bypassed in a future migration.
- Priority: Low while migrations are `&'static str` constants, but must be revisited in Phase 3 (History).

**`docs/architecture.md` describes an agent system, not the desktop app:**
- Files: `docs/architecture.md`
- Issue: The file describes a `Planner`, `Executor`, `Memory Writer`, `Memory Manager`, `Retriever`, `Judge` system. None of these exist in the codebase. The actual system is a Tauri v2 desktop app with a Svelte 5 frontend, SQLite persistence, typed IPC, and a modular Rust backend. The `CLAUDE.md` read order lists this file first, so every future agent will read a wrong architecture description before doing work.
- Fix approach: Rewrite `docs/architecture.md` before Phase 02 planning begins.

**`docs/threat-model.md` and `docs/privacy-boundaries.md` are stubs:**
- Files: `docs/threat-model.md`, `docs/privacy-boundaries.md`
- Issue: Both files contain only a header and a list of focus areas with no actual content. Phase 4 (Privacy) and the security module implementations depend on these as specifications. Provider routing decisions in Phase 2 also require the threat model.
- Fix approach: Populate both before Phase 02 begins. The threat model specifically needs content before provider routing is designed.

---

## Build / Runtime Gaps

**Phase 01 has never been runtime-verified:**
- Evidence: `01-VERIFICATION.md` status is `human_needed`. Five checks are deferred to human verification: app launch, keyboard navigation, screen reader behavior, persistence across restarts, and `cargo test`.
- Risk: Compilation errors, migration panics, or IPC wiring failures may exist and are invisible to static analysis. The verification environment had no Rust/Cargo toolchain.
- Required before Phase 02: Execute `cargo test --workspace --all-targets` and `npm run dev` in an environment with the full Tauri toolchain.

**`src-tauri/migrations/` directory is empty:**
- Files: `src-tauri/migrations/.gitkeep`
- Issue: All migrations are embedded as `&'static str` in `src-tauri/src/storage/migrations.rs`. The `migrations/` directory exists but holds only a `.gitkeep`. Phase 3 (History) will need a decision: keep migrations embedded in Rust or move them to `.sql` files in this directory. The empty directory creates ambiguity about the intended approach.

**Only 2 IPC commands registered; 6 IPC modules are unregistered stubs:**
- Files: `src-tauri/src/main.rs:51-54`, `src-tauri/src/ipc/chat.rs`, `history.rs`, `files.rs`, `providers.rs`, `privacy.rs`, `inventory.rs`
- Issue: `main.rs` registers only `get_active_surface` and `set_active_surface`. All other IPC modules export only non-command stub functions and are never passed to `invoke_handler`. When Phase 2 implements real commands, they must be added to both `invoke_handler` and `capabilities/main.json` before they become callable. There is no test or lint that enforces this sync.

**No CI pipeline confirmed:**
- Files: `.github/` (untracked directory — contents not confirmed)
- Risk: No automated build, test, or type-check runs on push. The `npm run check` and `cargo test` results documented in phase verification were produced manually inside the agent environment, not by CI.

---

## Phase Readiness: Phase 02 (Routing) Prerequisites

Phase 02 (ROUTE-01, ROUTE-02) requires the following before planning begins:

| Prerequisite | Status | Blocks |
|---|---|---|
| Phase 01 runtime-verified (`cargo test` + `npm run dev` pass) | NOT DONE | Regression risk if boot is broken |
| `docs/architecture.md` reflects actual Tauri/Svelte/Rust architecture | NOT DONE | Misleads routing design and future agent context |
| `docs/threat-model.md` has actual threat analysis | NOT DONE | Provider routing is a named threat area |
| `docs/provider-routing.md` is a sufficient spec for Phase 02 | Unchecked | Likely yes — review before planning |
| `src-tauri/src/security/secrets.rs` has an implementation plan | NOT DONE | ROUTE-01 requires backend-owned secrets |
| `src-tauri/src/providers/routing.rs` scaffolded to real types | NOT DONE | Core of Phase 02 |
| `src-tauri/src/providers/sse.rs` scaffolded to real types | NOT DONE | ROUTE-02 (streaming) |
| New IPC commands added to `invoke_handler` and `capabilities/main.json` | Pattern exists, not yet needed | Required for every new command in Phase 02 |

---

## Architectural Drift

**`docs/architecture.md` vs. actual implementation:**
- The file describes a `Planner`/`Executor`/`Memory` agent system. The actual system is a Tauri v2 + Svelte 5 desktop client with SQLite, typed IPC, and a Rust backend. This is a complete mismatch. The `CLAUDE.md` read order puts this file first, so it actively misleads every agent that reads it.

**`ipc/mod.rs` doc comment references a non-existent inventory file:**
- `src-tauri/src/ipc/mod.rs` states commands must stay in sync with `security/command-inventory.toml`. This file does not exist. The sync requirement is aspirational documentation with no enforcement.

**`docs/privacy-boundaries.md` is a stub, but the IPC layer already implements a boundary:**
- `src-tauri/src/ipc/app_shell.rs` enforces window-label checking as a backend-side privacy boundary. This is the right pattern, but it is undocumented at the spec level — `docs/privacy-boundaries.md` contains no boundary definitions to validate against.

---

## Test Coverage Gaps

**Rust integration tests not yet executed:**
- Files: `src-tauri/tests/app_shell.rs`
- What's not tested: 5 integration tests exist but have never been run. `tests/rust/` is empty (`tests/rust/.gitkeep`).
- Risk: Tests may have compile errors or runtime failures invisible to static analysis.
- Priority: High — must run before Phase 02 starts.

**No frontend unit tests:**
- Files: `src/lib/stores/surface.ts`, `src/lib/components/*.svelte`
- What's not tested: The Svelte 5 store logic (`hydrate()`, `setSurface()`, `normalizeIpcError()`) has no test suite. No test files exist under `src/lib/`.
- Risk: Store regressions surface only as runtime failures.
- Priority: Medium — add when Phase 02 adds store complexity.

**All E2E, security, and fixture test directories are empty:**
- Files: `tests/e2e/.gitkeep`, `tests/security/.gitkeep`, `tests/fixtures/.gitkeep`
- What's not tested: No E2E tests, no adversarial fixture tests, no security tests.
- Risk: REL-02 requires "adversarial fixture evidence" as a release gate. Nothing is in place.
- Priority: High for REL-02 (Phase 6), but must be planned from Phase 02 onwards.

---

*Concerns audit: 2026-06-13*
