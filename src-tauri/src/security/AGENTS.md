# security/AGENTS.md

This subtree owns secrets, redaction, sandboxing, and command policy.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/threat-model.md` — stub (header + 5-item focus-area list, no real analysis).
3. `../../../docs/privacy-boundaries.md` — stub; the one concrete fact in it is that `redaction.rs` was deleted (see below).
4. `../AGENTS.md`

## Purpose

Owns provider credential storage (`secrets.rs`, OS keychain via the `keyring` crate), opaque file-path tokenization (`file_tokens.rs`), artifact HTML sanitization/CSP wrapping (`artifact_sandbox.rs`), and the command/window allowlist (`command_policy.rs`). **There is no redaction module** — `redaction.rs` was deleted in commit `c7fffd1` as an unconditional stub with zero call sites; `mod.rs` no longer declares it.

## Contracts & Invariants

- In production builds, any keychain read failure (including `NoEntry`) is treated as `NotConfigured` — fail closed. The `dev-env-secrets` env-var fallback compiles only under `cfg(any(test, feature = "dev-env-secrets"))` and is absent from default release builds.
- `command_policy::COMMANDS` is a flat hardcoded `&[&str]` allowlist with a single `ALLOWED_WINDOW = "main"` constant — simplified from a prior per-command window table in commit `c7fffd1` "since only one window exists." Adding a second window in `tauri.conf.json` requires revisiting this.
- `policy_check` and the per-module `assert_main_window` helpers are NOT interchangeable: `policy_check` additionally validates the command name against `COMMANDS`; `assert_main_window` only checks the window label.
- `artifact_sandbox::sanitize` is deliberately narrow — script-block stripping, inline event-handler removal, `javascript:` URI neutralization via plain string scanning, not an HTML parser. It's explicitly scoped to "Phase 5 is static-preview only." Do not assume it's safe for richer rendering modes (live iframes, script execution) without re-auditing it as a general sanitizer.
- Security error enums (`SecretsError`, `FileTokenError`) follow the same `{code, message}` serde pattern as IPC errors but convert via explicit `From` impls in the calling `ipc::*` module rather than being returned directly.
- Treat secrets handling as a hard boundary; keep command policy separate from provider routing.

## Pitfalls

- `ProviderId` is documented as "exhaustive for Phase 2 (only OpenRouter is wired)." Adding a second provider requires extending both the enum and its `FromStr`/`account_label` match arms — there's no exhaustiveness check against an external provider registry.
- `get_provider_key`/`get_credential_status` fall back to the env-var key only in test/dev builds and only *after* a keychain error. In production, a corrupted/inaccessible keychain entry is indistinguishable from "never configured" (`NotConfigured`) — that's deliberate fail-closed design, not a bug.

