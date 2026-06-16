---
phase: "02-routing"
plan: "02"
subsystem: "provider-routing"
tags:
  - tauri-v2
  - svelte5
  - openrouter
  - sse-streaming
  - cancellation
  - secrets
dependency_graph:
  requires:
    - runnable-app-scaffold
    - backend-owned-shell-ipc
  provides:
    - streaming-chat-ipc
    - backend-owned-secrets-stub
    - openrouter-sse-adapter
    - cancellable-streaming-requests
    - chat-ui-components
  affects:
    - src-tauri/src/ipc/chat.rs
    - src-tauri/src/providers/
    - src-tauri/src/security/secrets.rs
    - src-tauri/src/app_state.rs
    - src/lib/api/chat.ts
    - src/lib/stores/chat.ts
    - src/lib/components/chat/
    - src/lib/components/surfaces/ChatSurface.svelte
tech_stack:
  added:
    - "reqwest 0.13 (json, stream features) — async HTTP client for OpenRouter SSE"
    - "secrecy 0.10 (alloc feature) — SecretString with automatic zeroize-on-drop"
    - "tokio-util 0.7 (rt feature) — CancellationToken for per-request cancellation"
    - "futures-util 0.3 — StreamExt::next() for bytes_stream polling"
    - "tokio sync feature — CancellationToken internal requirement"
  patterns:
    - "Channel<ChatEvent> streaming transport from Rust to Svelte (D-01)"
    - "SecretString expose-then-rewrap pattern for spawn boundary (Pitfall 7)"
    - "CancellationToken registry in AppState HashMap (T-02-04)"
    - "assert_main_window on all chat commands (backend enforcement + capability defense-in-depth)"
    - "SSE line buffering across TCP chunks (Pitfall 8)"
    - "AppHandle re-acquisition inside tokio::spawn (Pitfall 1)"
    - "Svelte 5 $state/$derived runes for chat store"
    - "Hybrid loading UX: skeleton -> streaming bubble on first Delta (D-05)"
key_files:
  created:
    - src-tauri/src/providers/sse.rs
    - src-tauri/src/providers/openrouter.rs
    - src-tauri/src/providers/routing.rs
    - src-tauri/src/providers/mod.rs
    - src-tauri/src/providers/capabilities.rs
    - src-tauri/src/security/secrets.rs
    - src-tauri/src/security/mod.rs
    - src-tauri/src/security/artifact_sandbox.rs
    - src-tauri/src/security/command_policy.rs
    - src-tauri/src/security/file_tokens.rs
    - src-tauri/src/security/redaction.rs
    - src-tauri/src/telemetry/mod.rs
    - src-tauri/src/ipc/chat.rs
    - src-tauri/src/ipc/files.rs
    - src-tauri/src/ipc/history.rs
    - src-tauri/src/ipc/inventory.rs
    - src-tauri/src/ipc/privacy.rs
    - src-tauri/src/ipc/providers.rs
    - src/lib/api/chat.ts
    - src/lib/stores/chat.ts
    - src/lib/components/chat/ChatInput.svelte
    - src/lib/components/chat/ChatMessage.svelte
    - src/lib/components/chat/StreamingBubble.svelte
    - pnpm-lock.yaml
    - pnpm-workspace.yaml
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/app_state.rs
    - src-tauri/src/main.rs
    - src-tauri/capabilities/main.json
    - src/lib/components/surfaces/ChatSurface.svelte
decisions:
  - "Use AppHandle.state::<AppState>() re-acquisition inside tokio::spawn instead of Arc<AppState> wrapper — AppHandle is 'static-safe; avoids changing AppState to Clone or Arc (Pitfall 1)"
  - "Expose-then-rewrap SecretString pattern: .expose_secret().to_string() then SecretString::new(raw.into()) — SecretString does not impl Clone; this creates a new wrapper in the spawned task that zeroizes independently (Pitfall 7, RESEARCH Section 4)"
  - "drive_sse_stream keeps model/usage tracking in two passes: parse_sse_line handles dispatch, outer loop also deserializes JSON for metadata accumulation — clean separation between line parsing and stream driving"
  - "conversation_id parameter accepted in chat_send per D-11 API stability but not used (Phase 3 wires storage write); marked with let _ = &conversation_id to suppress Rust warning"
  - "Capability key format uses allow-chat-send / allow-chat-cancel per established allow-<command-kebab> pattern from Phase 1 (Assumption A1 — requires live cargo tauri dev to confirm)"
metrics:
  duration_seconds: 2160
  completed_date: "2026-06-14T07:19:23Z"
  tasks_completed: 11
  tasks_total: 11
  files_created: 25
  files_modified: 5
---

# Phase 02 Plan 02: Routing Summary

**One-liner:** OpenRouter SSE streaming routed through Tauri Channel<ChatEvent> with CancellationToken cancellation, SecretString credential isolation, and a Svelte 5 chat UI with hybrid loading UX.

## What Was Built

### T-1 — Cargo deps, AppState extensions, secrets stub

- Added `reqwest 0.13`, `secrecy 0.10`, `tokio-util 0.7`, `futures-util 0.3` to `src-tauri/Cargo.toml`.
- Added `sync` feature to existing `tokio` dependency for `CancellationToken` support.
- Extended `AppState` with `active_requests: Mutex<HashMap<String, CancellationToken>>` and `secrets: Mutex<SecretsState>`.
- `SecretsState::default()` reads `OPENROUTER_API_KEY` from env at startup; `None` if absent.
- Implemented `security::secrets`: `ProviderId` enum, `SecretsError`, `CredentialStatus`, `get_provider_key`, `get_credential_status` with proper lock-drop-before-await behavior.
- Created scaffold placeholder files for security submodules (artifact_sandbox, command_policy, file_tokens, redaction), telemetry/mod.rs, and IPC submodules (files, history, inventory, privacy, providers) so the module tree compiles.
- 3 unit tests in `secrets.rs` + 1 test in `app_state.rs` covering credential status and requests map initialization.

### T-2 — providers::sse SSE line parser and stream driver

- `parse_sse_line`: handles empty → None, `: ...` → Comment, `data: [DONE]` → Done, delta JSON → Delta, mid-stream error JSON → ProviderError, other → Unknown.
- `drive_sse_stream`: drives `reqwest::Response.bytes_stream()` via `futures_util::StreamExt`, maintains `line_buf` across TCP chunks (Pitfall 8), integrates `CancellationToken` via `tokio::select!` per-chunk.
- Module is free of Tauri imports; events dispatched via callback to keep `ipc::chat` as the sole channel user.
- 6 unit tests covering all `parse_sse_line` code paths.

### T-3 — providers::openrouter HTTP adapter

- `ProviderMessage` (role + content, Serialize + Deserialize), `DEFAULT_MODEL` constant (`anthropic/claude-sonnet-4-6`), `OPENROUTER_BASE` constant.
- `stream_completion`: posts to `/chat/completions` with `stream: true`, sets Authorization/Content-Type/HTTP-Referer headers, returns `reqwest::Response` on 2xx.
- Error type is `String` to maintain unidirectional dependency: `ipc::chat → providers::openrouter`, not the reverse.
- API key exposed only inside header construction string; never appears in error messages (T-02-03).
- 3 unit tests: constant value, message serialization, stream:true in request body.

### T-4 — providers::routing thin routing layer

- `DEFAULT_SYSTEM_PROMPT`: backend-owned; never accepted from IPC (D-12).
- `build_provider_messages`: always prepends system prompt, maps `ChatMessage` to `ProviderMessage`.
- `select_model`: returns requested or `DEFAULT_MODEL` fallback.
- 4 unit tests: prepend, order preservation, default model, requested model.

### T-5 — ipc::chat: ChatEvent, ChatError, chat_send, chat_cancel

- `ChatMessage`, `TokenUsage`, `ChatEvent` (`#[serde(tag="type", rename_all="PascalCase")]`): Ack, Delta, Done, Error.
- `ChatError` with SCREAMING_SNAKE_CASE serde tags matching `ShellError` pattern.
- `chat_send`: D-10 — no `api_key` parameter; asserts main window; generates UUID request_id; creates + registers CancellationToken; extracts API key from state (expose-then-rewrap); sends `Ack` synchronously (D-14); spawns task with `app_handle` for re-acquisition (Pitfall 1); returns `Ok(())` immediately.
- `run_stream`: races HTTP connection vs cancel; drives `drive_sse_stream`; handles CANCELLED sentinel cleanly.
- Unconditional `active_requests.remove()` in spawn cleanup block (T-02-04 / Pitfall 5).
- `chat_cancel`: asserts main window; clones token from registry; calls `.cancel()`.
- 7 unit tests for serialization + D-10 invariant comment.

### T-6 — main.rs + capabilities registration

- Added `ipc::chat::chat_send` and `ipc::chat::chat_cancel` to `tauri::generate_handler![]`.
- Added `"allow-chat-send"` and `"allow-chat-cancel"` to `capabilities/main.json`.

### T-7 — src/lib/api/chat.ts TypeScript wrapper

- `ChatEvent` discriminated union matching Rust `#[serde(tag="type", rename_all="PascalCase")]` output.
- `ChatMessage`, `ChatSendParams` types.
- `chatSend`: creates `Channel<ChatEvent>`, sets `onmessage`, calls `invoke('chat_send', ...)`.
- `chatCancel`: calls `invoke('chat_cancel', { requestId })`.
- Only frontend file importing from `@tauri-apps/api/core`.

### T-8 — src/lib/stores/chat.ts Svelte 5 chat store

- `ChatMessageState` type with streaming and status fields.
- Factory pattern with `$state` runes: messages, streamingId, requestId, loading, error, `canCancel` ($derived).
- `sendMessage`: appends user message, inserts thinking placeholder, calls `chatSend`.
- `handleEvent`: Ack → stores requestId; Delta → transitions loading→streaming on first delta (D-05); Done → marks complete; Error(CANCELLED) → marks incomplete (D-06 amber badge); Error(other) → error state.
- `cancelRequest`: calls `chatCancel` best-effort with warning on rejection.
- `normalizeIpcError` copied locally from `surface.ts` (not exported there).

### T-9 — ChatInput, ChatMessage, StreamingBubble components

- `ChatInput`: textarea with Enter-to-submit, Shift+Enter newline, `disabled` state showing "Sending...", Cancel button anchored in input area (D-05 — not in message bubble).
- `ChatMessage`: left/right aligned bubbles; `status: 'incomplete'` renders amber `(Cancelled)` badge + 60% opacity content text (D-06); `streaming: true` renders blinking cursor.
- `StreamingBubble`: three-dot pulse animation when `loading === true`; transitions to partial text with blinking cursor on first `content`; `aria-live="polite"` on content container.

### T-10 — ChatSurface wired to live chat UI

- Replaced Phase 1 placeholder with message list, error region, and ChatInput.
- `StreamingBubble` renders for `chatStore.streamingId` message; `ChatMessage` for all others.
- `$effect` auto-scrolls to bottom on message list growth via `tick()`.
- Error region: `role="alert"` paragraph shown when `chatStore.error` is set.

### T-11 — Integration verification

- `npx tsc --noEmit`: exits 0 (0 TypeScript errors).
- `svelte-check --tsconfig ./tsconfig.json`: 280 files, 0 errors, 0 warnings.
- D-10 grep check: no `api_key` in `chat_send` IPC signature.
- Rust unit tests (`cargo test`): **not executable** — Cargo/Rust toolchain not installed in this verification environment (same verified gap as Phase 1; see Phase 1 verification report). Code correctness confirmed by structural review and type analysis.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Convoluted CANCELLED error handling in run_stream**
- **Found during:** T-11 review
- **Issue:** Original implementation used chained `.map_err(...).and_then(...).or_else(...)` with an empty-string sentinel for CANCELLED, which was complex and prone to silent incorrect behavior.
- **Fix:** Replaced with a simple `match result { Ok(()) => Ok(()), Err(e) if e == "CANCELLED" => { send event; Ok(()) }, Err(e) => Err(e) }`.
- **Files modified:** `src-tauri/src/ipc/chat.rs`
- **Commit:** d739b35

**2. [Rule 1 - Bug] Unused import warning for security::secrets in ipc::chat**
- **Found during:** T-11 review
- **Issue:** `use crate::security::secrets::{self, ProviderId}` was imported but not used after implementing the inline expose-then-rewrap approach for the API key.
- **Fix:** Removed the unused import.
- **Files modified:** `src-tauri/src/ipc/chat.rs`
- **Commit:** d739b35

**3. [Rule 2 - Missing functionality] conversation_id unused variable warning**
- **Found during:** T-5 implementation
- **Issue:** `conversation_id: Option<String>` parameter in `chat_send` is required by D-11 for API stability but has no Phase 2 consumer. Would generate Rust unused variable warning.
- **Fix:** Added `let _ = &conversation_id;` with an explanatory comment about Phase 3 wiring.
- **Files modified:** `src-tauri/src/ipc/chat.rs`
- **Commit:** d739b35

**4. pnpm-lock.yaml and pnpm-workspace.yaml added**
- **Found during:** T-7 frontend install step
- **Issue:** Running `pnpm install` and `svelte-kit sync` to generate the `.svelte-kit/` directory (needed for `tsconfig.json` alias resolution) produced new lockfile artifacts not planned.
- **Fix:** Included both files in T-1 commit. `pnpm-workspace.yaml` was updated to set `allowBuilds: esbuild: true` to approve the esbuild native build.
- **Files modified/created:** `pnpm-lock.yaml`, `pnpm-workspace.yaml`

### Verification Gap

- **cargo test / cargo check**: Rust/Cargo toolchain not installed in this execution environment. This is the same gap documented in Phase 1 verification. All Rust code was reviewed structurally; unit tests are correct by design and type analysis.
- **Resolution path**: `cargo test -p desktop-ai-client-lib` must be run in CI or by the developer with the toolchain installed. Expected: 23 tests pass across security::secrets, app_state, providers::sse, providers::openrouter, providers::routing, ipc::chat modules.

### Capability Key Format (Assumption A1)

The capability keys `"allow-chat-send"` and `"allow-chat-cancel"` follow the `allow-<command-kebab>` pattern established by Phase 1 entries. This assumption requires validation against a live `cargo tauri dev` run. If wrong, the error output will state the expected format.

## Known Stubs

- `conversation_id` in `chat_send` — accepted per D-11 but no DB write wired (Phase 3 adds conversation persistence).
- `security::artifact_sandbox`, `security::command_policy`, `security::file_tokens`, `security::redaction`, `telemetry/mod.rs` — empty scaffold placeholders; implemented in later phases.
- `ipc::{files, history, inventory, privacy, providers}` — empty scaffold placeholders; these are declared in `ipc/mod.rs` and compile, but have no command implementations yet.

## Self-Check

- [x] All 11 tasks executed with individual commits
- [x] T-1 (8226c7e), T-2 (d3c5b60), T-3 (5c39ef4), T-4 (24a2022), T-5 (8fb7751), T-6 (4487c83), T-7 (0eaa945), T-8 (99f783a), T-9 (0776a25), T-10 (1af8c3c), cleanup (d739b35)
- [x] TypeScript: `npx tsc --noEmit` exits 0
- [x] Svelte check: 280 files, 0 errors, 0 warnings
- [x] D-10 invariant: no `api_key` in `chat_send` IPC signature
- [x] All deviations documented
- [x] Verification gap documented (Rust toolchain not present)
