# Phase 4: Privacy - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-15
**Phase:** 04-privacy
**Areas discussed:** Secrets backing store, File access flow, Redaction scope, ipc::privacy surface

---

## Secrets Backing Store

| Option                                 | Description                                                                                     | Selected |
| -------------------------------------- | ----------------------------------------------------------------------------------------------- | -------- |
| OS keychain (keyring crate)            | macOS Keychain, Windows Credential Manager, Linux SecretService. Simpler dep, native UX.        | ✓        |
| tauri-plugin-stronghold                | IOTA Stronghold encrypted vault. Heavier dependency, requires snapshot file and pin derivation. |          |
| Hybrid: OS keychain + env-var fallback | OS keychain primary; env-var if unavailable. Keeps developer ergonomics from Phase 2.           |          |

**User's choice:** `KeyringSecretStore` via `keyring` crate. `EnvSecretStore` retained behind explicit dev/test config flag for CI/local dev only. Production must fail closed.

---

| Option                       | Description                                                                                               | Selected |
| ---------------------------- | --------------------------------------------------------------------------------------------------------- | -------- |
| IPC command from Settings UI | `privacy_set_provider_key` lets user paste key in Settings. Backend stores; frontend never sees it again. | ✓        |
| Env var at first boot only   | Migration step: read env var on first launch, write to keychain, discard. Dev flag keeps fallback.        |          |
| You decide                   | Claude picks write path.                                                                                  |          |

**User's choice:** Settings UI calls `privacy_set_provider_key` once. Rust stores in OS keychain. Optional env-var import behind explicit dev/test/migration flag; never silently in production.

---

| Option                                    | Description                                                                              | Selected |
| ----------------------------------------- | ---------------------------------------------------------------------------------------- | -------- |
| Yes — implement command_policy this phase | Runtime IPC authorization layer. ARCHITECTURE.md data flow shows it's needed for SEC-01. | ✓        |
| No — defer to Phase 6 (Release)           | Phase 6 inventory enforcement layer. Focus Phase 4 on SEC-01/02/03 only.                 |          |

**User's choice:** `security::command_policy` in scope for Phase 4 as a static runtime allow-table. Full deny-by-inventory release verifier deferred to Phase 6.

---

## File Access Flow

| Option                     | Description                                                                                     | Selected |
| -------------------------- | ----------------------------------------------------------------------------------------------- | -------- |
| Tauri native file dialog   | Backend calls tauri-plugin-dialog; Rust gets path, mints token, returns token only.             | ✓        |
| Frontend sends path string | Renderer uses web file input, sends path to `files_request_token`. Backend validates/tokenizes. |          |

**User's choice:** Rust-owned native file selection. Backend receives path directly from OS dialog, validates, mints opaque token, returns token + safe metadata only. `chat_send` accepts attachment tokens, never filesystem paths.

---

| Option                       | Description                                                                            | Selected |
| ---------------------------- | -------------------------------------------------------------------------------------- | -------- |
| Session-scoped in-memory map | `HashMap` in `AppState`; tokens dropped on app quit. Simple, no disk exposure.         | ✓        |
| Persisted with TTL           | Tokens in SQLite with expiry. Survive restarts. More complex, creates storage surface. |          |

**User's choice:** Session-scoped in-memory only. Safe attachment metadata can be persisted later, never token authority or source paths.

---

| Option                           | Description                                                                                | Selected |
| -------------------------------- | ------------------------------------------------------------------------------------------ | -------- |
| Full file token system           | `security::file_tokens` + `ipc::files` + extend `chat_send` with attachment tokens.        |          |
| Token layer only, no chat wiring | Build `security::file_tokens` + minimal `ipc::files`; defer `chat_send` wiring to Phase 5. | ✓        |

**User's choice:** Build `security::file_tokens` and minimal `ipc::files` now; defer `chat_send` attachment wiring until Phase 5.

---

## Redaction Scope

| Option                           | Description                                                            | Selected     |
| -------------------------------- | ---------------------------------------------------------------------- | ------------ |
| Secrets + paths + prompt content | Redact three categories: API keys, file paths, message/prompt content. | ✓ (expanded) |
| Secrets + paths only             | Redact keys and paths; prompt content not logged at all.               |              |
| You decide                       | Claude sets categories from threat model.                              |              |

**User's choice:** Option 1 expanded — secrets, paths, and all content-bearing data (prompt/message/attachment content). Prompt and message content normally omitted entirely; redaction is the safety net.

---

| Option                       | Description                                                              | Selected |
| ---------------------------- | ------------------------------------------------------------------------ | -------- |
| Append-only file, JSON Lines | One JSON object per line in app data dir. Structured, no extra DB table. | ✓        |
| SQLite table                 | New `audit_events` table. Reuses pool; couples audit log to DB health.   |          |
| You decide                   | Claude picks format and location.                                        |          |

**User's choice:** Append-only JSON Lines file in app data logs directory. Schema validation + redaction before every write. SQLite `audit_events` deferred.

---

| Option                          | Description                                                   | Selected |
| ------------------------------- | ------------------------------------------------------------- | -------- |
| No debug escape hatch           | Redaction unconditional. Dev mode can add metadata logs only. | ✓        |
| Yes — dev flag lowers redaction | Debug build logs IPC payloads (but never raw secrets).        |          |

**User's choice:** No debug escape hatch. Redaction is unconditional. Dev mode may emit metadata-level logs (command names, timestamps, status codes) only.

---

## ipc::privacy Surface

| Option                               | Description                                                                                               | Selected |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------- | -------- |
| Two commands: set + status           | `privacy_set_provider_key` + `privacy_get_credential_status`. Clear deferred; overwrite handles rotation. |          |
| Three commands: set + status + clear | Add `privacy_clear_provider_key` for revocation/removal.                                                  | ✓        |

**User's choice:** Three commands — set, status, and clear. No frontend command ever reads the key value. Re-setting handles rotation; clear handles revocation.

---

| Option              | Description                                                                                  | Selected |
| ------------------- | -------------------------------------------------------------------------------------------- | -------- |
| IPC layer only      | Phase 4 implements backend commands only. SettingsSurface.svelte stays placeholder.          |          |
| Minimal Settings UI | Add narrow credential management UI (set/clear/status) to SettingsSurface.svelte this phase. | ✓        |

**User's choice:** Minimal Settings UI in Phase 4 for credential management only. Broader Settings experience deferred.

---

| Option                                       | Description                                                                                          | Selected |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------- | -------- |
| Window-label + caller validation per command | `command_policy::check()` verifies window label + no forbidden params. Extends `assert_main_window`. |          |
| Static allow/deny table                      | Hardcoded allow-table mapping command names to allowed callers. Sets up Phase 6 inventory early.     | ✓        |

**User's choice:** Static runtime allow-table in `command_policy.rs`. Every IPC handler calls it before executing. Forbidden payload fields prevented by typed structs with `deny_unknown_fields`. Phase 6 replaces/augments with inventory verifier.

---

## Claude's Discretion

- Keyring service name and account label format for keychain entries
- Exact safe metadata fields returned by `files_open_dialog` (filename, size, MIME type expected)
- JSON Lines schema for audit log entries (per event type: command name, timestamp, window label, status code — no payload content)
- Error enum naming for `PrivacyError` and `FilesError` variants
- In-memory token ID type (`uuid::Uuid` or newtype wrapper)

## Deferred Ideas

- `chat_send` attachment wiring — token parameter deferred to Phase 5
- Persisted attachment metadata — safe metadata storage deferred to Phase 5
- `security::artifact_sandbox` — Phase 5
- Full Settings experience (model selection, retention config, theme) — future settings phase
- SQLite `audit_events` table — deferred; JSON Lines file is sufficient for Phase 4
- `privacy_import_from_env` helper — dev/test utility, not an IPC command
- Phase 6 command-inventory verifier — `command_policy` allow-table is runtime enforcement; inventory release gate is Phase 6
