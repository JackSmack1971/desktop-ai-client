---
phase: 4
slug: privacy
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-06-15
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | `src-tauri/Cargo.toml` |
| **Quick run command** | `cargo test --manifest-path src-tauri/Cargo.toml -- security` |
| **Full suite command** | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --manifest-path src-tauri/Cargo.toml -- <module>::tests`
- **After every plan wave:** Run `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Req ID | Behavior | Test Type | Automated Command | File Exists | Status |
|--------|----------|-----------|-------------------|-------------|--------|
| SEC-01 | `get_provider_key` returns `NotConfigured` when keychain empty | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- secrets::tests` | ✅ (existing) | ⬜ pending |
| SEC-01 | `get_provider_key` returns key after `store_provider_key` (mocked keyring) | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- secrets::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-01 | `privacy_set_provider_key` never exposes key in IPC response | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::privacy::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-01 | `PrivacyError` serializes with SCREAMING_SNAKE_CASE code field | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::privacy::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-01 | `command_policy` denies unknown commands | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- security::command_policy::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-01 | `command_policy` denies wrong window label | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- security::command_policy::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-02 | `files_open_dialog` returns `Cancelled` when no file selected | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::files::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-02 | Token resolves to path after mint, fails after revoke | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- security::file_tokens::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-02 | `FilesError` serializes with SCREAMING_SNAKE_CASE code field | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- ipc::files::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-03 | `redact_path` replaces path with `[REDACTED]` | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- security::redaction::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-03 | `redact_secret` replaces API key with `[REDACTED]` | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- security::redaction::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-03 | Audit log entry does not contain path or secret | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- telemetry::audit_log::tests` | ❌ Wave 0 | ⬜ pending |
| SEC-03 | `AuditEntry` serializes to valid JSON (parseable) | unit | `cargo test --manifest-path src-tauri/Cargo.toml -- telemetry::audit_log::tests` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src-tauri/src/security/secrets.rs` — test for keyring-backed store (using mock keyring in test cfg or `#[cfg(test)]` mock trait)
- [ ] `src-tauri/src/security/file_tokens.rs` — tests for `mint_token`, `resolve_token`, `revoke_token`
- [ ] `src-tauri/src/security/redaction.rs` — tests for all three redaction categories (secrets, paths, content)
- [ ] `src-tauri/src/security/command_policy.rs` — tests for allow-table enforcement (deny unknown, deny wrong label, allow correct label)
- [ ] `src-tauri/src/ipc/privacy.rs` — error serialization tests (no Tauri runtime needed)
- [ ] `src-tauri/src/ipc/files.rs` — error serialization tests + FilesError variants
- [ ] `src-tauri/src/telemetry/audit_log.rs` — JSON Lines structure + redaction verification
- [ ] `src-tauri/src/app_state.rs` — update `make_state_with_secrets()` helper to include `file_tokens`

**Keyring mock note:** For CI testing without a real OS keychain, use `#[cfg(test)]` mock trait or `keyring`'s mock credential store (if available). The planner must address mock strategy in Wave 0.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| OS keychain write/read on Windows Credential Manager | SEC-01 | Requires live Windows OS keychain | Run app, enter key in SettingsSurface, close app, reopen, verify CredentialStatus=CONFIGURED |
| Native file picker dialog opens correctly | SEC-02 | Requires OS desktop environment | Run app, invoke `files_open_dialog` via SettingsSurface or DevTools console, verify dialog appears |
| Audit log file created at correct path | SEC-03 | Path resolution depends on app bundle identity | Run app, invoke any privacy command, check `%LOCALAPPDATA%\<bundle>\logs\audit.log` (Windows) |

---

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING (❌) references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
