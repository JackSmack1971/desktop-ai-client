# Plan 004: Cap attachment read size and restrict to text-like MIME types before forwarding to the provider

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- src-tauri/src/ipc/chat.rs`
> If `chat.rs` changed since this plan was written, compare the "Current
> state" excerpt below against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: security
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

`read_attachment` in `src-tauri/src/ipc/chat.rs` reads the *entire* contents of any file the user attaches into memory, with no size ceiling, and splices it directly into the system-prompt-adjacent message sent to OpenRouter. There's no explicit type allow-list either — anything that isn't sniffed as `text/*`, `*/json`, or `*/xml` falls through to `String::from_utf8_lossy`, which silently mangles binary content (images, PDFs, executables) into a "readable" string rather than rejecting it. Two distinct problems: (1) a very large file causes an unbounded read into process memory and an unbounded outbound HTTP payload to a third-party API, and (2) non-text files get garbage-decoded and shipped to the provider instead of being rejected with a clear error.

## Current state

`src-tauri/src/ipc/chat.rs:540-557` (`read_attachment`, full function as it exists today):

```rust
fn read_attachment(path: &Path) -> Result<String, ChatError> {
    let mime = from_path(path).first_or_octet_stream();
    let mime_type = mime.type_().as_str();
    let mime_subtype = mime.subtype().as_str();
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment");

    let content = if mime_type == "text" || mime_subtype == "json" || mime_subtype == "xml" {
        fs::read_to_string(path).map_err(|e| ChatError::CredentialError(e.to_string()))?
    } else {
        String::from_utf8_lossy(&fs::read(path).map_err(|e| ChatError::CredentialError(e.to_string()))?)
            .into_owned()
    };

    Ok(format!("Filename: {filename}\nMIME: {mime}\nContent:\n{content}"))
}
```

Note the existing (slightly mis-named, but pre-existing — do not "fix" it as part of this plan) pattern of mapping I/O errors to `ChatError::CredentialError` — keep using that same variant for the new error cases this plan introduces, for consistency with the rest of this function; do not invent a new `ChatError` variant.

`ChatError` (lines 87-100) currently has no specific "attachment too large" or "unsupported attachment type" variant — `CredentialError(String)` is the closest existing bucket used for this function's other failure modes, so reuse it (matching the existing style, even though the name is a slight misnomer for this case — introducing a more precisely-named variant is a larger API change than this plan's scope justifies).

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0, no errors |
| Run tests | `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` | all pass |

## Scope

**In scope** (the only file you should modify):
- `src-tauri/src/ipc/chat.rs` — only the `read_attachment` function and its test module.

**Out of scope** (do NOT touch, even though related):
- `src-tauri/src/ipc/files.rs` (`files_read_token`) — a separate read path with separate UX requirements (e.g. it may legitimately need to read non-text files for preview purposes); do not add the same restriction there without a separate design decision.
- Frontend file-picker UI (`ChatInput.svelte` or any attachment-picker component) — adding a client-side size warning before upload is a reasonable follow-up but is a UX change, not this bug fix; out of scope here.
- The MIME-sniffing library/logic itself (`mime_guess::from_path`) — keep using it as-is; this plan only adds a size check and tightens what happens when the type isn't text-like.

## Git workflow

- Branch: `advisor/004-cap-attachment-size-and-type`
- Commit message: `fix(chat): cap attachment size and reject non-text MIME types`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Add a size constant and a pre-read size check

At the top of `src-tauri/src/ipc/chat.rs`, near the other top-level constants/imports (after the `use` block, before `ChatMessage`), add:

```rust
/// Maximum bytes read from a single attachment before it is rejected. Chosen
/// to keep outbound provider payloads bounded; revisit if legitimate use
/// cases need larger attachments (would need provider-side chunking too).
const MAX_ATTACHMENT_BYTES: u64 = 2 * 1024 * 1024; // 2 MiB
```

### Step 2: Rewrite `read_attachment` to check size before reading, and reject non-text MIME types instead of lossy-decoding them

Replace the full body of `read_attachment` (lines 540-557) with:

```rust
fn read_attachment(path: &Path) -> Result<String, ChatError> {
    let mime = from_path(path).first_or_octet_stream();
    let mime_type = mime.type_().as_str();
    let mime_subtype = mime.subtype().as_str();
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("attachment");

    let metadata = fs::metadata(path).map_err(|e| ChatError::CredentialError(e.to_string()))?;
    if metadata.len() > MAX_ATTACHMENT_BYTES {
        return Err(ChatError::CredentialError(format!(
            "attachment '{filename}' is {} bytes, exceeds the {MAX_ATTACHMENT_BYTES}-byte limit",
            metadata.len()
        )));
    }

    let is_text_like = mime_type == "text" || mime_subtype == "json" || mime_subtype == "xml";
    if !is_text_like {
        return Err(ChatError::CredentialError(format!(
            "attachment '{filename}' has unsupported type '{mime}' — only text, JSON, and XML attachments are supported"
        )));
    }

    let content = fs::read_to_string(path).map_err(|e| ChatError::CredentialError(e.to_string()))?;

    Ok(format!("Filename: {filename}\nMIME: {mime}\nContent:\n{content}"))
}
```

Key changes from the current version: (1) a `fs::metadata` size check runs before any content read, (2) non-text-like files are now a hard `Err` instead of falling through to `String::from_utf8_lossy`, which means the `else` branch and its lossy-decode call are removed entirely.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0.

## Test plan

Add to `src-tauri/src/ipc/chat.rs`'s existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn read_attachment_rejects_file_over_size_limit() {
    use std::io::Write;
    let path = std::env::temp_dir().join(format!("oversized-{}.txt", Uuid::new_v4()));
    {
        let mut f = std::fs::File::create(&path).expect("create temp file");
        let chunk = vec![b'a'; 1024 * 1024]; // 1 MiB chunk
        for _ in 0..3 {
            f.write_all(&chunk).expect("write chunk"); // 3 MiB total, over the 2 MiB limit
        }
    }
    let result = read_attachment(&path);
    let _ = std::fs::remove_file(&path);
    assert!(
        matches!(result, Err(ChatError::CredentialError(_))),
        "expected oversized attachment to be rejected, got: {result:?}"
    );
}

#[test]
fn read_attachment_rejects_non_text_mime_type() {
    let path = std::env::temp_dir().join(format!("binary-{}.bin", Uuid::new_v4()));
    std::fs::write(&path, [0xFFu8, 0xFE, 0x00, 0x01]).expect("write binary file");
    let result = read_attachment(&path);
    let _ = std::fs::remove_file(&path);
    assert!(
        matches!(result, Err(ChatError::CredentialError(_))),
        "expected non-text attachment to be rejected, got: {result:?}"
    );
}

#[test]
fn read_attachment_accepts_small_text_file() {
    let path = std::env::temp_dir().join(format!("ok-{}.txt", Uuid::new_v4()));
    std::fs::write(&path, "hello world").expect("write text file");
    let result = read_attachment(&path);
    let _ = std::fs::remove_file(&path);
    assert!(result.is_ok(), "expected small text attachment to succeed: {result:?}");
    assert!(result.unwrap().contains("hello world"));
}
```

These model the existing test style in this file (plain function calls, `assert!`/`matches!`, no mocking framework needed since `read_attachment` only touches the filesystem).

Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` → all pass, including the 3 new tests.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` exits 0, all tests pass including the 3 new ones
- [ ] `read_attachment` rejects files over `MAX_ATTACHMENT_BYTES` before reading their content
- [ ] `read_attachment` rejects non-text-like MIME types instead of lossy-decoding them
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- `read_attachment`'s signature or surrounding code no longer matches the excerpt above.
- You determine 2 MiB is clearly wrong for this product's actual use case (e.g. there's a documented requirement elsewhere for larger attachments) — report instead of guessing a different number; this plan's constant is a reasonable default, not a researched requirement.

## Maintenance notes

- If a future feature needs larger attachments (e.g. whole-codebase context), this hard size cap and the text-only restriction will need revisiting — likely via chunking/summarization rather than just raising the constant, since the real constraint is the provider's own context window, not just this byte limit.
- A reviewer should check that the frontend surfaces these new `CredentialError` messages clearly to the user (the existing `normalizeIpcError` in `src/lib/stores/chat.ts` already surfaces `message` from any IPC error object generically, so no frontend change should be needed — verify this in the PR review rather than assuming).
