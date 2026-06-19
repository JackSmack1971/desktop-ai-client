# Plan 003: Revoke file tokens after their content is read so the token map can't grow unbounded

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- src-tauri/src/security/file_tokens.rs src-tauri/src/ipc/chat.rs src-tauri/src/ipc/files.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

`security/file_tokens.rs` defines `mint_token`, `resolve_token`, and `revoke_token` — a one-time-use token pattern for letting the frontend reference a backend-resolved file path without ever seeing the raw path. `mint_token` is called every time the user picks a file (`ipc/files.rs:93`, `files_open_dialog`). `resolve_token` is called when an attachment's content is actually read into a chat prompt (`ipc/chat.rs:525`, inside `resolve_attachments`). But `revoke_token` — which exists and is unit-tested (`file_tokens.rs:55`) — has **zero callers anywhere in production code**. Every minted token stays in `AppState.file_tokens: Mutex<HashMap<Uuid, PathBuf>>` for the entire app session.

This is the same class of bug the codebase explicitly already fixed elsewhere: `chat.rs`'s own doc comment (lines 12-13) calls out that `active_requests` cleanup is unconditional specifically "to prevent HashMap growth (STRIDE T-02-04)" — the file-token map has no equivalent cleanup. A long session with many file attachments leaks memory for the life of the process and retains stale path mappings (a minor data-residency concern: paths linger in memory after the content has already been consumed).

## Current state

- `src-tauri/src/security/file_tokens.rs` (full file, 83 lines) — `revoke_token` already exists:
  ```rust
  pub fn revoke_token(state: &AppState, token: Uuid) -> Result<(), FileTokenError> {
      let mut guard = state
          .file_tokens
          .lock()
          .map_err(|e| FileTokenError::LockPoisoned(e.to_string()))?;
      guard.remove(&token);
      Ok(())
  }
  ```
  Confirmed via `grep -rn "revoke_token" src-tauri/src` that its only callers today are its own unit test (`mint_resolve_revoke_round_trip`, line 50-60).
- `src-tauri/src/ipc/chat.rs:511-538` (`resolve_attachments`) — where tokens are resolved and consumed:
  ```rust
  fn resolve_attachments(
      state: &tauri::State<'_, AppState>,
      attachments: Option<Vec<Uuid>>,
  ) -> Result<Option<String>, ChatError> {
      let Some(tokens) = attachments else {
          return Ok(None);
      };
      if tokens.is_empty() {
          return Ok(None);
      }
      let mut rendered = Vec::new();
      for token in tokens {
          let path = crate::security::file_tokens::resolve_token(&state, token)
              .map_err(|e| ChatError::CredentialError(e.to_string()))?;
          rendered.push(read_attachment(&path)?);
      }
      let mut body = String::from("Attached file context:\n");
      for attachment in rendered {
          body.push_str("\n---\n");
          body.push_str(&attachment);
          body.push('\n');
      }
      Ok(Some(body))
  }
  ```
  This is called once from `chat_send` at `chat.rs:234`, with `attachments: Option<Vec<Uuid>>` being the full list of tokens for the current message. Each token in the loop is read exactly once (its content is inlined into `rendered`), so once the loop body for a given `token` completes successfully, that token will never be needed again by this function.
- `src-tauri/src/ipc/files.rs:102-122` (`files_read_token`) — a second, separate consumer of `resolve_token` (not `resolve_attachments`): this command lets the frontend read raw file bytes for non-chat purposes (e.g. previewing an attachment chip before sending). This command currently has no revoke either, and unlike the chat path it's plausible the frontend calls it more than once for the same token (e.g. re-rendering a preview) — so revoking here unconditionally would be a behavior change beyond this bug's scope. **Do not add revoke to `files_read_token`** — see Scope below.

## Commands you will need

| Purpose       | Command                                            | Expected on success |
| ------------- | -------------------------------------------------- | ------------------- |
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0, no errors   |
| Run tests     | `cargo test --manifest-path src-tauri/Cargo.toml`  | all tests pass      |

## Scope

**In scope** (the only files you should modify):

- `src-tauri/src/ipc/chat.rs` — add a revoke call inside `resolve_attachments`, after each token's content has been successfully read.

**Out of scope** (do NOT touch, even though related):

- `src-tauri/src/ipc/files.rs` / `files_read_token` — do not add revoke here. This command may legitimately be called more than once for the same token (e.g. a frontend preview that re-reads before the user decides whether to attach it to a message). Revoking on first read would break that flow. If the team wants token lifecycle management for this path too, that's a separate, larger design decision (e.g. "revoke when the attachment chip is removed from the composer" needs a new IPC command) — out of scope here.
- `src-tauri/src/security/file_tokens.rs` — `revoke_token` itself is correct as written; no changes needed.
- Any change to when/how `mint_token` is called — out of scope.

## Git workflow

- Branch: `advisor/003-revoke-file-tokens-after-use`
- Commit message: `fix(security): revoke file tokens after attachment content is read`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Revoke each token in `resolve_attachments` after its content is successfully read

In `src-tauri/src/ipc/chat.rs`, modify the loop inside `resolve_attachments` (currently lines 523-528):

```rust
let mut rendered = Vec::new();
for token in tokens {
    let path = crate::security::file_tokens::resolve_token(&state, token)
        .map_err(|e| ChatError::CredentialError(e.to_string()))?;
    rendered.push(read_attachment(&path)?);
}
```

to:

```rust
let mut rendered = Vec::new();
for token in tokens {
    let path = crate::security::file_tokens::resolve_token(&state, token)
        .map_err(|e| ChatError::CredentialError(e.to_string()))?;
    rendered.push(read_attachment(&path)?);
    // Single-use: the content has been read into `rendered`, so the token
    // is no longer needed. Revoke even though this is best-effort (a lock
    // failure here should not fail the whole chat_send call).
    let _ = crate::security::file_tokens::revoke_token(&state, token);
}
```

Place the revoke call _after_ `read_attachment(&path)?` succeeds (i.e., after the `rendered.push(...)` line), not before — if `read_attachment` fails (propagating the `?` and aborting the whole function), the token should remain valid so the caller could plausibly retry with the same token rather than having it silently vanish on a transient read failure.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0.

## Test plan

Add one new test to `src-tauri/src/ipc/chat.rs`'s existing `#[cfg(test)] mod tests` block (after the existing `title_from_messages` tests). Model it after the round-trip test in `security/file_tokens.rs:50-60`, but driving it through `resolve_attachments` so the integration is what's actually tested, not just the underlying primitive:

```rust
#[test]
fn resolve_attachments_revokes_token_after_reading_content() {
    use crate::app_state::AppState;
    use crate::security::file_tokens;
    use std::io::Write;

    let state_inner = AppState::default();
    let state = tauri::State::from(&state_inner); // adjust to however other tests in
                                                    // this codebase construct a tauri::State
                                                    // for a unit test — check storage/sqlite.rs
                                                    // tests for the established pattern first;
                                                    // if tauri::State cannot be constructed
                                                    // directly in a unit test in this Tauri
                                                    // version, call resolve_attachments's
                                                    // logic via file_tokens::resolve_token +
                                                    // file_tokens::revoke_token directly against
                                                    // AppState instead, asserting the same
                                                    // round-trip behavior `resolve_attachments`
                                                    // is supposed to produce.

    let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
    write!(tmp, "hello attachment").unwrap();
    let token = file_tokens::mint_token(&state_inner, tmp.path().to_path_buf()).expect("mint");

    let result = resolve_attachments(&state, Some(vec![token]));
    assert!(result.is_ok(), "resolve_attachments should succeed: {result:?}");

    // The token must now be gone.
    let resolved_again = file_tokens::resolve_token(&state_inner, token);
    assert!(
        matches!(resolved_again, Err(file_tokens::FileTokenError::NotFound(_))),
        "expected token to be revoked after use, got: {resolved_again:?}"
    );
}
```

**Note for the executor**: `tauri::State` cannot always be constructed directly from a plain value outside a running `tauri::App` in every Tauri version — check how existing tests in this codebase handle this (search `grep -rn "tauri::State::from\|fn resolve_attachments\|#\[cfg(test)\]" src-tauri/src/ipc/chat.rs` and look at how `storage/sqlite.rs`'s tests construct pool/state fixtures). If `resolve_attachments`'s signature (`state: &tauri::State<'_, AppState>`) makes direct unit testing impractical in this Tauri version, it is acceptable to test the underlying primitive instead: call `file_tokens::mint_token`, then `file_tokens::resolve_token`, then manually replicate the revoke-after-read sequence against a plain `&AppState`, and assert the post-condition (token gone after the sequence). Do not skip the test — pick whichever of these two approaches actually compiles, and prefer testing through `resolve_attachments` if `tauri::State` construction works.

If a `tempfile` dev-dependency does not already exist in `Cargo.toml`, do not add a new dependency for this one test — instead write the test fixture file directly into the OS temp dir using `std::env::temp_dir()` and a UUID-suffixed filename, then delete it at the end of the test (or rely on OS temp-dir cleanup, matching whatever pattern `ipc::files.rs`'s existing tests use, if any touch real files — if none do, prefer `std::env::temp_dir()` over adding `tempfile`).

Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` → all pass, including the new test.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` exits 0, includes the new revoke-after-read test, passing
- [ ] `resolve_attachments` in `chat.rs` calls `file_tokens::revoke_token` after each successful `read_attachment`
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `resolve_attachments`'s signature or loop structure no longer matches the excerpt above — re-read the live file first.
- Constructing a `tauri::State<'_, AppState>` for a unit test turns out to require infrastructure this codebase doesn't have and you cannot find a working pattern in existing tests — report back with what you tried rather than adding a new test-only dependency or refactoring `resolve_attachments`'s signature to make it more testable (that refactor, if wanted, should be its own plan).
- You find evidence that some other in-progress code path calls `resolve_attachments` with the _same_ token twice for legitimately different requests (e.g. a retry mechanism) — that would mean revoking on first use breaks a real flow; if so, stop and report instead of revoking.

## Maintenance notes

- This only closes the leak for the chat-attachment path. `files_read_token` (out of scope here) still mints tokens with no revoke path at all — if attachment previewing becomes a heavier feature, revisit whether that command needs its own lifecycle (e.g. revoke when the user removes the attachment chip in the composer before sending, which would need a new `files_revoke_token` IPC command).
- If `chat_send` is ever changed to allow the _same_ attachment token to be referenced by more than one in-flight request (not the case today), this revoke-on-first-read approach would break the second request. Flag that as a design constraint if it comes up.
