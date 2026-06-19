# Plan 002: Route every IPC command through the shared `command_policy::policy_check`

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- src-tauri/src/ipc/ src-tauri/src/security/command_policy.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none
- **Category**: security
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

`src-tauri/src/security/command_policy.rs` defines a `COMMANDS` allow-list and a single `policy_check(command, window_label)` function that is meant to be the one place that enforces "this command may only be invoked from the main window." But today only 3 of 6 IPC modules actually call it (`privacy.rs`, `files.rs`, `artifacts.rs`). The other 3 (`app_shell.rs`, `chat.rs`, `history.rs` — 8 of the 15 listed commands) each define their own private, near-identical `assert_main_window` function and never call `command_policy::policy_check` at all, even though every one of those command names is present in `COMMANDS`.

The observable behavior is identical today (every local check also pins to `"main"`), so this is not an active vulnerability. But it means: the allow-list table is dead code for those 8 commands — a reviewer trusting `command_policy.rs` as the enforcement gate would be wrong; the `UnknownCommand` rejection path never executes for those commands, so a typo'd/renamed name in `COMMANDS` would never be caught; and the same window-auth logic now lives in 4 separate places with no shared test tying them together. A future change to tighten or loosen the policy (e.g. adding a second allowed window for a debug build) would need to be replicated in 4 places to stay consistent, and nothing would fail the build if one were missed.

## Current state

- `src-tauri/src/security/command_policy.rs` (full file, 79 lines) — the canonical implementation:
  ```rust
  const ALLOWED_WINDOW: &str = "main";
  const COMMANDS: &[&str] = &[
      "get_active_surface", "set_active_surface", "chat_send", "chat_cancel",
      "history_list", "history_get", "history_delete", "history_search",
      "privacy_set_provider_key", "privacy_get_credential_status", "privacy_clear_provider_key",
      "files_open_dialog", "files_read_token", "artifact_get", "artifact_dismiss",
  ];
  pub fn policy_check(command: &str, window_label: &str) -> Result<(), PolicyError> {
      if !COMMANDS.contains(&command) {
          return Err(PolicyError::UnknownCommand(command.to_string()));
      }
      if window_label == ALLOWED_WINDOW { Ok(()) } else { Err(PolicyError::UnauthorizedWindow(window_label.to_string())) }
  }
  ```
- `src-tauri/src/ipc/app_shell.rs:91-101` — local duplicate to remove:
  ```rust
  fn assert_main_window(window: &tauri::Window) -> Result<(), ShellError> {
      if window.label() != "main" {
          return Err(ShellError::UnauthorizedWindow(format!(
              "shell commands require the main window, got {:?}", window.label()
          )));
      }
      Ok(())
  }
  ```
  Called at `app_shell.rs:43` (`get_active_surface`) and `app_shell.rs:75` (`set_active_surface`).
- `src-tauri/src/ipc/chat.rs:115-125` — same pattern, called at `chat.rs:163` (`chat_send`) and `chat.rs:362` (`chat_cancel`). `ChatError` already has an `UnauthorizedWindow(String)` variant (line 91).
- `src-tauri/src/ipc/history.rs:74-82` — same pattern, called at `history.rs:93,122,165,184` (`history_list`, `history_get`, `history_delete`, `history_search`). `HistoryError` already has an `UnauthorizedWindow(String)` variant (line 27).
- **The exemplar to copy** — `src-tauri/src/ipc/privacy.rs:18-27` already does this correctly:
  ```rust
  impl From<command_policy::PolicyError> for PrivacyError {
      fn from(value: command_policy::PolicyError) -> Self {
          match value {
              command_policy::PolicyError::UnauthorizedWindow(msg) => {
                  PrivacyError::UnauthorizedWindow(msg)
              }
              command_policy::PolicyError::UnknownCommand(msg) => PrivacyError::PolicyViolation(msg),
          }
      }
  }
  ```
  and then each command body starts with `command_policy::policy_check("privacy_set_provider_key", window.label())?;` (line 60) — the `?` operator uses the `From` impl automatically.
- Note: `artifacts.rs:49-50` and `artifacts.rs:80-81` currently call **both** `command_policy::policy_check(...)` and the local `assert_main_window(&window)?` back-to-back — fully redundant once policy_check is in place. This plan removes that redundancy too (see Step 5).
- None of `ShellError`, `ChatError`, `HistoryError` currently have a `PolicyViolation`-equivalent variant for the `UnknownCommand` case — they all already have `UnauthorizedWindow(String)`, but mapping `PolicyError::UnknownCommand` needs _some_ variant. Reuse the existing `StorageError(String)` variant on `ShellError`/`HistoryError` for this (matching how `ArtifactError::from` maps `UnknownCommand` to `ArtifactError::StorageError` at `artifacts.rs:21` — an existing precedent in this codebase, slightly odd naming but consistent). For `ChatError`, reuse `ProviderError(String)` (no `StorageError` variant exists there) since it's the closest existing "unexpected backend condition" bucket.

## Commands you will need

| Purpose       | Command                                            | Expected on success |
| ------------- | -------------------------------------------------- | ------------------- |
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0, no errors   |
| Run tests     | `cargo test --manifest-path src-tauri/Cargo.toml`  | all tests pass      |

## Scope

**In scope** (the only files you should modify):

- `src-tauri/src/ipc/app_shell.rs`
- `src-tauri/src/ipc/chat.rs`
- `src-tauri/src/ipc/history.rs`
- `src-tauri/src/ipc/artifacts.rs` (remove the now-fully-redundant local `assert_main_window` calls only — see Step 5)

**Out of scope** (do NOT touch, even though related):

- `src-tauri/src/security/command_policy.rs` — its `COMMANDS`/`ALLOWED_WINDOW`/`policy_check` implementation is correct and is the thing every other file should converge onto; do not change its logic.
- `src-tauri/src/ipc/privacy.rs` and `src-tauri/src/ipc/files.rs` — already correct, used as the exemplar; no changes needed.
- `security/command-inventory.toml` and `src-tauri/capabilities/main.json` — capability/inventory files are a separate defense-in-depth layer; this plan only consolidates the Rust-side window check, not the capability manifest.

## Git workflow

- Branch: `advisor/002-consolidate-command-policy`
- Commit per file (4 commits), message style: `fix(security): route <module> through command_policy::policy_check`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: `app_shell.rs` — add `From<PolicyError>`, call `policy_check`, delete `assert_main_window`

In `src-tauri/src/ipc/app_shell.rs`:

1. Add `use crate::security::command_policy;` to the imports (alongside the existing `use crate::app_state::{AppState, Surface};` and `use crate::storage::sqlite::ShellPreferenceStore;`).
2. Add, right after the `ShellError` enum definition:
   ```rust
   impl From<command_policy::PolicyError> for ShellError {
       fn from(value: command_policy::PolicyError) -> Self {
           match value {
               command_policy::PolicyError::UnauthorizedWindow(msg) => {
                   ShellError::UnauthorizedWindow(msg)
               }
               command_policy::PolicyError::UnknownCommand(msg) => {
                   ShellError::StorageError(msg)
               }
           }
       }
   }
   ```
3. In `get_active_surface` (line 43) and `set_active_surface` (line 75), replace `assert_main_window(&window)?;` with `command_policy::policy_check("get_active_surface", window.label())?;` and `command_policy::policy_check("set_active_surface", window.label())?;` respectively — match each call to its own command name, not a copy-paste of the same string.
4. Delete the `assert_main_window` function (lines 91-101) entirely.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0, no errors, no warnings about unused `assert_main_window`.

### Step 2: `chat.rs` — same pattern

In `src-tauri/src/ipc/chat.rs`:

1. Add `use crate::security::command_policy;` to the imports.
2. Add, right after the `ChatError` enum definition (after line 100):
   ```rust
   impl From<command_policy::PolicyError> for ChatError {
       fn from(value: command_policy::PolicyError) -> Self {
           match value {
               command_policy::PolicyError::UnauthorizedWindow(msg) => {
                   ChatError::UnauthorizedWindow(msg)
               }
               command_policy::PolicyError::UnknownCommand(msg) => {
                   ChatError::ProviderError(msg)
               }
           }
       }
   }
   ```
3. In `chat_send` (line 163) replace `assert_main_window(&window)?;` with `command_policy::policy_check("chat_send", window.label())?;`.
4. In `chat_cancel` (line 362) replace `assert_main_window(&window)?;` with `command_policy::policy_check("chat_cancel", window.label())?;`.
5. Delete the `assert_main_window` function (lines 115-125).

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0.

### Step 3: `history.rs` — same pattern, 4 call sites

In `src-tauri/src/ipc/history.rs`:

1. Add `use crate::security::command_policy;` to the imports.
2. Add, right after the `HistoryError` enum definition (after line 29):
   ```rust
   impl From<command_policy::PolicyError> for HistoryError {
       fn from(value: command_policy::PolicyError) -> Self {
           match value {
               command_policy::PolicyError::UnauthorizedWindow(msg) => {
                   HistoryError::UnauthorizedWindow(msg)
               }
               command_policy::PolicyError::UnknownCommand(msg) => {
                   HistoryError::StorageError(msg)
               }
           }
       }
   }
   ```
3. Replace each `assert_main_window(&window)?;` call with the matching `command_policy::policy_check("<command_name>", window.label())?;`:
   - line 93 → `"history_list"`
   - line 122 → `"history_get"`
   - line 165 → `"history_delete"`
   - line 184 → `"history_search"`
4. Delete the `assert_main_window` function (lines 74-82).

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0.

### Step 4: Update or remove tests that referenced the deleted `assert_main_window` functions

None of `app_shell.rs`, `chat.rs`, or `history.rs`'s existing `#[cfg(test)]` blocks call `assert_main_window` directly (they only test error serialization and pure helpers like `title_from_messages`) — confirm this by reading each file's test module before deleting the function in Steps 1-3. If you find a test that does call it directly, update that test to call `command_policy::policy_check("<command_name>", "main")` instead, matching the existing tests in `command_policy.rs:46-49`.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::` → all tests in the `ipc` module pass.

### Step 5: `artifacts.rs` — remove the now-fully-redundant local check

`src-tauri/src/ipc/artifacts.rs` already calls `command_policy::policy_check(...)` (lines 49, 80) immediately followed by the local `assert_main_window(&window)?` (lines 50, 81) — both checks are now redundant with each other since `policy_check` already enforces the same `window.label() == "main"` condition. Remove the `assert_main_window(&window)?;` line at 50 and 81, and delete the `assert_main_window` function (lines 33-41). Leave the `command_policy::policy_check(...)` calls in place — they're already correct.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0. `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::artifacts` → all tests pass.

### Step 6: Full workspace verification

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml` → all tests pass, including the existing `command_policy.rs` test suite (`allows_known_command_for_main_window`, `rejects_wrong_window`, `rejects_unknown_command`, `allows_artifact_commands_for_main_window`, `serializes_code_field`) which is unaffected by this change but should still pass.

## Test plan

Add one new test to `src-tauri/src/security/command_policy.rs`'s existing `#[cfg(test)] mod tests` block, asserting the table itself stays in sync with what's registered (this is the regression guard that justifies the whole consolidation):

```rust
#[test]
fn every_table_command_is_a_real_registered_command() {
    // This is a documentation test: COMMANDS should list exactly the
    // #[tauri::command] names registered in lib.rs's generate_handler!.
    // It cannot introspect the handler list at compile time, so this test
    // exists to be updated by hand whenever a command is added/removed —
    // if you're adding a command, add it here too.
    let expected = [
        "get_active_surface", "set_active_surface", "chat_send", "chat_cancel",
        "history_list", "history_get", "history_delete", "history_search",
        "privacy_set_provider_key", "privacy_get_credential_status", "privacy_clear_provider_key",
        "files_open_dialog", "files_read_token", "artifact_get", "artifact_dismiss",
    ];
    for cmd in expected {
        assert!(COMMANDS.contains(&cmd), "expected {cmd} in COMMANDS");
    }
    assert_eq!(COMMANDS.len(), expected.len(), "COMMANDS list drifted from expected set");
}
```

Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib security::command_policy` → all pass including the new test.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` exits 0, all tests pass
- [ ] `grep -rn "fn assert_main_window" src-tauri/src/ipc/` returns no matches (all 4 local copies deleted)
- [ ] `grep -rn "command_policy::policy_check" src-tauri/src/ipc/` shows exactly 15 call sites (one per command in `COMMANDS`)
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- Any of `app_shell.rs`, `chat.rs`, `history.rs`, `artifacts.rs` no longer matches the line numbers/structure described above — re-read the live file before guessing.
- A test you cannot identify breaks after deleting one of the `assert_main_window` functions — do not delete a function that a test still references without first updating that test.
- You discover a 5th IPC module (beyond the 6 named here) that also has its own local window-check duplicate — report it; it likely needs the same fix but wasn't in scope for this plan's recon.

## Maintenance notes

- After this lands, `command_policy::COMMANDS` is genuinely the single source of truth — a reviewer auditing window-auth coverage only needs to read one file.
- If a new IPC command is added in the future, the contributor must: (1) add it to `COMMANDS` in `command_policy.rs`, (2) call `command_policy::policy_check("<name>", window.label())?` as the first line of the handler, and (3) add a `From<command_policy::PolicyError>` impl for that module's error type if one doesn't already exist. The new test added in this plan's Test plan section will catch a missing `COMMANDS` entry, but not a missing `policy_check()` call inside a handler — that gap is acceptable for now (no test infra exists to catch it without a much larger integration-test investment; flag it as a candidate after `plans/005-add-test-coverage.md` lands).
