# Phase 2: Routing — Plan

**Phase:** 02-routing
**Goal:** Route user prompts through deterministic provider selection and robust streaming transport
**Mode:** mvp
**Requirements:** ROUTE-01, ROUTE-02

---

## Phase Goal

**As a** desktop AI client user, **I want to** send a prompt and receive a streamed response from an AI provider, **so that** I can have a real conversation without the app holding my API key.

---

## Success Criteria

1. A prompt can be routed without frontend ownership of provider secrets.
2. Streaming output arrives in order and preserves partial output.
3. Cancellation and typed error handling work without corrupting the active stream.

---

## Locked Decision Coverage

| Decision                                          | Covered In                                |
| ------------------------------------------------- | ----------------------------------------- |
| D-01 `Channel<ChatEvent>` transport               | T-3 (ipc::chat)                           |
| D-02 `ChatEvent` tagged enum                      | T-3 (ipc::chat)                           |
| D-03 terminal state via channel only              | T-3 (run_stream), T-8 (store)             |
| D-04 `chat_cancel` + `CANCELLED` code             | T-4 (ipc::chat)                           |
| D-05 hybrid loading UX                            | T-9 (StreamingBubble), T-10 (ChatSurface) |
| D-06 cancel amber badge + incomplete status       | T-9 (StreamingBubble)                     |
| D-07 thin secrets stub                            | T-2 (security::secrets)                   |
| D-08 `get_provider_key` + `get_credential_status` | T-2 (security::secrets)                   |
| D-09 env-var backing in AppState                  | T-1 (app_state + Cargo)                   |
| D-10 no `api_key` IPC param                       | T-3 (`cargo check` gate)                  |
| D-11 `chat_send` parameter shape                  | T-3 (ipc::chat)                           |
| D-12 system prompt backend-owned                  | T-3 (routing.rs prepends system prompt)   |
| D-13 default model constant                       | T-3 (`DEFAULT_MODEL` constant)            |
| D-14 `request_id` via `ChatEvent::Ack`            | T-3 (Ack sent before spawn)               |

---

## Threat Model

### Trust Boundaries

| Boundary                  | Description                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Renderer → IPC            | All `chat_send` / `chat_cancel` input is untrusted; backend enforces window label, validates types          |
| IPC → OpenRouter          | API key never crosses IPC; `SecretString` held in `AppState` behind Mutex; exposed only inside spawned task |
| SSE bytes → parsed events | Raw HTTP bytes treated as potentially malformed; parse errors are non-fatal (skip chunk, log warning)       |
| Spawned task → channel    | `channel.send()` errors after terminal event are ignored, not panicked                                      |

### STRIDE Threat Register

| Threat ID | Category               | Component                               | Disposition | Mitigation Plan                                                                                                              |
| --------- | ---------------------- | --------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------- |
| T-02-01   | Information Disclosure | `chat_send` IPC signature               | mitigate    | D-10 invariant: no `api_key` param; enforced by type system; `cargo check` is the gate                                       |
| T-02-02   | Tampering              | System prompt via `chat_send`           | mitigate    | D-12: system prompt built in `providers::routing`, never accepted from IPC payload                                           |
| T-02-03   | Information Disclosure | `ChatError` messages                    | mitigate    | Error variants must not format secret values; `ChatError::CredentialError("not configured")` — never `format!("{key}")`      |
| T-02-04   | Denial of Service      | `active_requests` HashMap growth        | mitigate    | Unconditional cleanup in spawned task `finally`-equivalent block (Pitfall 5 from RESEARCH.md)                                |
| T-02-05   | Elevation of Privilege | `chat_send` called from non-main window | mitigate    | `assert_main_window(&window)` as first statement in both `chat_send` and `chat_cancel`                                       |
| T-02-06   | Information Disclosure | `SecretString` in debug logs            | mitigate    | `secrecy::SecretString` redacts in `Debug`/`Display` by design; never call `.expose_secret()` in log macros                  |
| T-02-07   | Tampering              | Malformed SSE mid-stream error          | mitigate    | SSE parser checks for top-level `"error"` key in chunk JSON; emits `ChatEvent::Error` on detection                           |
| T-02-SC   | Tampering              | Cargo dependency installs               | accept      | All four new crates verified against crates.io (see Package Legitimacy Audit in RESEARCH.md); no `[SUS]` or `[SLOP]` entries |

---

## Wave Structure

| Wave | Tasks     | Description                                   | Parallel? |
| ---- | --------- | --------------------------------------------- | --------- |
| 1    | T-1       | Cargo deps + AppState + secrets stub          | —         |
| 2    | T-2, T-3  | SSE parser + openrouter adapter (parallel)    | Yes       |
| 3    | T-4, T-5  | routing layer + ipc::chat (parallel)          | Yes       |
| 4    | T-6       | main.rs + capabilities registration           | —         |
| 5    | T-7, T-8  | Frontend API wrapper + chat store (parallel)  | Yes       |
| 6    | T-9, T-10 | UI components + ChatSurface wiring (parallel) | Yes       |
| 7    | T-11      | Integration compile + test verification       | —         |

---

## Tasks

---

### Wave 1: Foundation

#### T-1 — Extend Cargo.toml and AppState for streaming and secrets

**Files:**

- `src-tauri/Cargo.toml`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/security/secrets.rs`

**Description:**

Add the four new Cargo dependencies (per D-09, D-01 research findings). In `Cargo.toml` under `[dependencies]`:

- Add `reqwest = { version = "0.13", features = ["json", "stream"] }` — HTTP client with SSE stream support.
- Add `secrecy = { version = "0.10", features = ["alloc"] }` — `SecretString` wrapper with automatic zeroize-on-drop.
- Add `tokio-util = { version = "0.7", features = ["rt"] }` — provides `CancellationToken`.
- Add `futures-util = "0.3"` — `StreamExt::next()` for bytes_stream polling.
- Modify existing `tokio` entry to add `"sync"` feature: `tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }`.

Extend `src-tauri/src/app_state.rs` — keep existing `ShellState`, `Surface`, and their `impl` blocks intact. Add:

```
use std::collections::HashMap;
use tokio_util::sync::CancellationToken;
```

Add two new public fields to `AppState`:

- `pub active_requests: Mutex<HashMap<String, CancellationToken>>` — per D-04, stores the cancel token keyed by `request_id`.
- `pub secrets: Mutex<SecretsState>` — per D-09, holds the env-var-backed key.

Add `SecretsState` struct with one field: `pub openrouter_key: Option<secrecy::SecretString>`.

Implement `Default` for `SecretsState`: read `OPENROUTER_API_KEY` from environment via `std::env::var("OPENROUTER_API_KEY").ok()`, wrap the `String` in `secrecy::SecretString::new(v.into())` if present, else `None`. This read happens at app startup (when `AppState::default()` is called in `main.rs`).

Update `AppState::default()` to initialize the new fields:

- `active_requests: Mutex::new(HashMap::new())`
- `secrets: Mutex::new(SecretsState::default())`

Implement `security::secrets` stub (per D-07, D-08). Replace the scaffold placeholder in `src-tauri/src/security/secrets.rs` with:

- `pub enum ProviderId { OpenRouter }` — typed provider identifier; exhaustive for Phase 2.
- `pub enum SecretsError { NotConfigured(String), LockPoisoned(String) }` with `thiserror::Error` and `serde::Serialize` derives using the same `#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]` pattern from `ShellError`.
- `pub enum CredentialStatus { Configured, Missing }` with `serde::Serialize` derive.
- `pub fn get_provider_key(state: &crate::app_state::AppState, provider: ProviderId) -> Result<secrecy::SecretString, SecretsError>` — locks `state.secrets`, calls `.expose_secret()` on the stored key, clones the `String`, drops the guard, then returns `SecretString::new(cloned.into())`. Returns `SecretsError::NotConfigured` when the key is `None`. Never holds the lock across any await.
- `pub fn get_credential_status(state: &crate::app_state::AppState, provider: ProviderId) -> CredentialStatus` — locks `state.secrets`, returns `CredentialStatus::Configured` if `openrouter_key.is_some()`, else `CredentialStatus::Missing`. Drops lock immediately.

Add `#[cfg(test)]` tests inline in `secrets.rs`:

- `get_credential_status_returns_missing_when_no_key` — construct `AppState` with `SecretsState { openrouter_key: None }`, assert `CredentialStatus::Missing`.
- `get_credential_status_returns_configured_when_key_present` — construct with a test key, assert `CredentialStatus::Configured`.
- `get_provider_key_returns_not_configured_when_missing` — assert `Err(SecretsError::NotConfigured(_))`.

Add a test to `app_state.rs`:

- `app_state_initializes_active_requests_empty` — `AppState::default()` then lock `active_requests` and assert `is_empty()`.

**Verification:**

```
cargo test -p desktop-ai-client-lib security::secrets
cargo test -p desktop-ai-client-lib app_state
cargo check -p desktop-ai-client-lib
```

**Done:** `cargo check` passes with the new deps resolved; `security::secrets` tests pass; `active_requests` and `secrets` fields present on `AppState`.

**Depends on:** (none — Wave 1 foundation)

---

### Wave 2: SSE Parser + OpenRouter Adapter (parallel)

#### T-2 — Implement providers::sse SSE line parser

**Files:**

- `src-tauri/src/providers/sse.rs`

**Description:**

Replace the scaffold placeholder with a complete SSE parser. No live network access is needed; the parser operates on `&[u8]` slices for unit testing.

Define these types (all with `Debug` derive):

- `pub struct SseUsage { pub prompt_tokens: u32, pub completion_tokens: u32 }` — `Deserialize + Serialize + Clone`.
- Private `SseChunk` struct: `model: Option<String>`, `usage: Option<SseUsage>`, `choices: Vec<SseChoice>`, `error: Option<SseChunkError>` — `Deserialize`.
- Private `SseChoice`: `delta: Option<SseDelta>`, `finish_reason: Option<String>` — `Deserialize`.
- Private `SseDelta`: `content: Option<String>`, `role: Option<String>` — `Deserialize`.
- Private `SseChunkError`: `message: String`, `code: Option<serde_json::Value>` — `Deserialize`. Presence of this field signals a mid-stream provider error.
- `pub enum SseEvent { Delta { text: String }, Done { usage: Option<SseUsage>, model: String }, ProviderError { message: String }, Comment, Unknown }`.

Implement `pub fn parse_sse_line(line: &str) -> Option<SseEvent>`:

- Returns `None` for empty lines.
- Lines starting with `:` → `Some(SseEvent::Comment)`.
- Lines not starting with `data: ` → `Some(SseEvent::Unknown)`.
- `data: [DONE]` (trimmed) → `Some(SseEvent::Done { usage: None, model: String::new() })` — sentinel; caller supplies final model/usage.
- Otherwise: `serde_json::from_str::<SseChunk>(data_payload)`:
  - If `chunk.error.is_some()` → `Some(SseEvent::ProviderError { message: chunk.error.unwrap().message })`.
  - Otherwise collect non-empty `delta.content` values across `choices`; return `Some(SseEvent::Delta { text })` for non-empty content; `None` if nothing useful.
  - Parse errors: `log::warn!` and return `None` (non-fatal).

Implement `pub async fn drive_sse_stream(response: reqwest::Response, cancel_token: tokio_util::sync::CancellationToken, on_event: impl FnMut(SseEvent) -> Result<(), String> + Send + 'static) -> Result<(), String>`:

- Uses `futures_util::StreamExt` on `response.bytes_stream()`.
- Maintains `line_buf: String` across chunks — never split per-chunk.
- Splits `line_buf` on `\n`; trims trailing `\r`.
- Calls `parse_sse_line` on each complete line; dispatches to `on_event` for `Delta` and `ProviderError`; on `Done` fills in tracked `final_model` and `final_usage` then calls `on_event(SseEvent::Done { ... })` and returns `Ok(())`.
- Per-chunk `tokio::select!` with `cancel_token.cancelled()` branch — when cancelled, returns `Err("CANCELLED".to_string())`. Caller (`ipc::chat`) converts this to `ChatEvent::Error { code: "CANCELLED", ... }`.
- `channel.send()` is not called here — `drive_sse_stream` returns events to caller via callback, keeping the module free of Tauri imports.

Add `#[cfg(test)]` tests:

- `parse_sse_line_extracts_delta_content` — call with `data: {"choices":[{"delta":{"content":"Hello"}}]}` line, assert `SseEvent::Delta { text }` where `text == "Hello"`.
- `parse_sse_line_ignores_comment_lines` — call with `: OPENROUTER PROCESSING`, assert `SseEvent::Comment`.
- `parse_sse_line_handles_done_sentinel` — call with `data: [DONE]`, assert `SseEvent::Done { .. }`.
- `parse_sse_line_returns_none_for_empty` — call with `""`, assert `None`.
- `parse_sse_line_detects_mid_stream_error` — call with `data: {"error":{"message":"rate limited","code":429}}`, assert `SseEvent::ProviderError { .. }`.
- `parse_sse_line_skips_delta_with_empty_content` — call with `data: {"choices":[{"delta":{"content":""}}]}`, assert `None`.

**Verification:**

```
cargo test -p desktop-ai-client-lib providers::sse
```

**Done:** All six SSE parser unit tests pass; `drive_sse_stream` compiles without Tauri imports.

**Depends on:** T-1 (Cargo deps must resolve before `reqwest` and `tokio_util` are available)

---

#### T-3 — Implement providers::openrouter HTTP adapter

**Files:**

- `src-tauri/src/providers/openrouter.rs`

**Description:**

Replace the scaffold placeholder. This module builds the reqwest request and returns a `reqwest::Response`; it does not parse SSE bytes (that is `providers::sse`'s responsibility).

Define private `ChatCompletionRequest<'a>` struct (per RESEARCH.md Section 2):

- `model: &'a str`, `messages: &'a [ProviderMessage]`, `stream: bool`, `max_completion_tokens: Option<u32>` (skip_serializing_if None), `temperature: Option<f32>` (skip_serializing_if None) — `serde::Serialize`.

Define `pub struct ProviderMessage { pub role: String, pub content: String }` — `Serialize + Deserialize + Debug + Clone`.

Define `pub const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4-6"` — per D-13.

Define `pub const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1"`.

Implement `pub async fn stream_completion(client: &reqwest::Client, api_key: &secrecy::SecretString, model: &str, messages: &[ProviderMessage], max_completion_tokens: Option<u32>, temperature: Option<f32>) -> Result<reqwest::Response, String>`:

- Constructs `ChatCompletionRequest` with `stream: true`.
- Posts to `{OPENROUTER_BASE}/chat/completions`.
- Sets headers: `Authorization: Bearer {api_key.expose_secret()}`, `Content-Type: application/json`, `HTTP-Referer: https://desktop-ai-client` (per RESEARCH.md Section 9 — attribution header; no `X-Title` that could leak hostname).
- On non-2xx status: returns `Err(format!("HTTP {status}"))`.
- On success: returns `Ok(response)`.
- Error type is `String` so the module has no dependency on `ChatError` (dependency direction: `ipc::chat` → `providers::openrouter`, not the reverse).

Add `#[cfg(test)]` tests:

- `default_model_is_correct` — assert `DEFAULT_MODEL == "anthropic/claude-sonnet-4-6"` (per D-13; compile-time constant, verified at test time).
- `provider_message_serializes_role_and_content` — construct `ProviderMessage { role: "user".into(), content: "hi".into() }`, serialize to JSON, assert contains `"role":"user"` and `"content":"hi"`.
- `chat_completion_request_sets_stream_true` — construct a `ChatCompletionRequest`, serialize, assert `"stream":true`.

**Verification:**

```
cargo test -p desktop-ai-client-lib providers::openrouter
cargo check -p desktop-ai-client-lib
```

**Done:** Three unit tests pass; `stream_completion` signature compiles with `secrecy::SecretString` parameter.

**Depends on:** T-1 (Cargo deps)

---

### Wave 3: Routing Layer + IPC Commands (parallel)

#### T-4 — Implement providers::routing thin routing layer

**Files:**

- `src-tauri/src/providers/routing.rs`

**Description:**

Replace the scaffold placeholder. This module is thin for Phase 2 — capability-based selection is deferred. Its responsibility is: (1) select provider (always OpenRouter for Phase 2), (2) prepend the backend-owned system prompt (per D-12), (3) convert `ChatMessage` to `ProviderMessage` vec.

Define `pub const DEFAULT_SYSTEM_PROMPT: &str` — a short, backend-owned assistant persona string (e.g., `"You are a helpful AI assistant."`). This is never accepted from IPC (per D-12).

Implement `pub fn build_provider_messages(system_prompt: &str, messages: &[crate::ipc::chat::ChatMessage]) -> Vec<crate::providers::openrouter::ProviderMessage>`:

- Prepends `ProviderMessage { role: "system".into(), content: system_prompt.into() }`.
- Maps each `ChatMessage` to `ProviderMessage { role: msg.role.clone(), content: msg.content.clone() }`.
- Returns the combined vec.

Implement `pub fn select_model(requested: Option<&str>) -> String`:

- Returns `requested.unwrap_or(crate::providers::openrouter::DEFAULT_MODEL).to_string()`.

Add `#[cfg(test)]` tests:

- `build_provider_messages_prepends_system_prompt` — call with one user message, assert first element has `role == "system"` and second has `role == "user"`.
- `build_provider_messages_preserves_order` — call with `[user, assistant, user]`, assert output has `[system, user, assistant, user]`.
- `select_model_returns_default_when_none` — assert equals `DEFAULT_MODEL`.
- `select_model_returns_requested_when_some` — pass `"some/other-model"`, assert returned unchanged.

**Verification:**

```
cargo test -p desktop-ai-client-lib providers::routing
```

**Done:** Four routing unit tests pass; system prompt prepend is verified.

**Depends on:** T-3 (needs `ProviderMessage` type from `openrouter.rs`)

---

#### T-5 — Implement ipc::chat: ChatEvent, ChatError, ChatMessage, chat_send, chat_cancel

**Files:**

- `src-tauri/src/ipc/chat.rs`

**Description:**

Replace the scaffold placeholder with the full IPC command surface. This is the most complex task in the phase — every implementation detail from RESEARCH.md Sections 1, 3, and the pitfall list applies here.

**Types to define:**

`ChatMessage` (per D-11): `pub struct ChatMessage { pub role: String, pub content: String }` — `Deserialize + Serialize + Debug + Clone`.

`TokenUsage`: `pub struct TokenUsage { pub prompt_tokens: u32, pub completion_tokens: u32 }` — `Serialize + Deserialize + Debug + Clone`.

`ChatEvent` (per D-02): `#[derive(Debug, Clone, serde::Serialize)]` with `#[serde(tag = "type", rename_all = "PascalCase")]`:

- `Ack { request_id: String }` — per D-14, sent before spawning the streaming task.
- `Delta { text: String }` — incremental token.
- `Done { usage: Option<TokenUsage>, model: String }` — terminal success.
- `Error { code: String, message: String }` — terminal failure or cancellation.

`ChatError` (follow `ShellError` pattern): `#[derive(Debug, thiserror::Error, serde::Serialize)]` with `#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]`:

- `UnauthorizedWindow(String)` — from `assert_main_window`.
- `CredentialError(String)` — key missing or lock poisoned.
- `ProviderError(String)` — HTTP or stream error.
- `ChannelError(String)` — `channel.send()` failure on pre-terminal events.
- `RequestNotFound(String)` — `chat_cancel` called with unknown `request_id`.

**`chat_send` implementation (per D-01, D-03, D-11, D-14):**

Signature (CRITICAL — do NOT add `api_key` parameter per D-10):

```
pub async fn chat_send(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, crate::app_state::AppState>,
    messages: Vec<ChatMessage>,
    model: Option<String>,
    conversation_id: Option<String>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    channel: tauri::ipc::Channel<ChatEvent>,
) -> Result<(), ChatError>
```

Body:

1. `assert_main_window(&window)?` — first line, always.
2. Generate `request_id = uuid::Uuid::new_v4().to_string()`.
3. Create `let token = tokio_util::sync::CancellationToken::new()`.
4. Register: lock `state.active_requests`, insert `(request_id.clone(), token.clone())`, drop lock — never hold across await.
5. Extract API key: lock `state.secrets`, call `security::secrets::get_provider_key(&state, ProviderId::OpenRouter)` (or inline the expose-then-rewrap pattern), clone to owned `SecretString`, drop lock — per Pitfall 3 and Pitfall 7.
6. Send `ChatEvent::Ack { request_id: request_id.clone() }` via `channel.send()` — per D-14. Map error to `ChatError::ChannelError`. This must happen synchronously before `tokio::spawn`.
7. Resolve model: `providers::routing::select_model(model.as_deref())`.
8. Build provider messages: `providers::routing::build_provider_messages(providers::routing::DEFAULT_SYSTEM_PROMPT, &messages)` — per D-12.
9. Capture `state.inner()` as `Arc`-based handle for the spawn: use `app_handle` to re-acquire state inside the task (per Pitfall 1 from RESEARCH.md — `tauri::State<'_>` cannot cross spawn boundary). Inside the spawned closure use `app_handle.state::<crate::app_state::AppState>()`.
10. `tokio::spawn(async move { ... })` — moves: `api_key`, `resolved_model`, `provider_messages`, `channel`, `token.clone()`, `request_id.clone()`, `app_handle`.
11. Inside spawn: call `run_stream(...)` (private async fn), then unconditionally remove `request_id` from `active_requests` regardless of result (Pitfall 5). On `Err(e)` from `run_stream`, send `ChatEvent::Error { code: "PROVIDER_ERROR".into(), message: e }` — ignore channel send error after terminal event.
12. Return `Ok(())` from `chat_send` immediately after spawn.

`run_stream` private async fn:

- Creates `reqwest::Client::new()`.
- `tokio::select!` — races `providers::openrouter::stream_completion(...)` against `cancel_token.cancelled()`. On cancel: send `ChatEvent::Error { code: "CANCELLED", ... }` and return `Ok(())`.
- On success: calls `providers::sse::drive_sse_stream(response, cancel_token, on_event_callback)` where the callback maps `SseEvent::Delta { text }` → `channel.send(ChatEvent::Delta { text })`, `SseEvent::Done { usage, model }` → `channel.send(ChatEvent::Done { ... })`, `SseEvent::ProviderError { message }` → `channel.send(ChatEvent::Error { code: "PROVIDER_ERROR", message })`.
- Returns `Err(e)` if `drive_sse_stream` returns `Err`.

**`chat_cancel` implementation (per D-04):**

Signature: `pub async fn chat_cancel(window: tauri::Window, state: tauri::State<'_, crate::app_state::AppState>, request_id: String) -> Result<(), ChatError>`

Body:

1. `assert_main_window(&window)?` — first line.
2. Lock `state.active_requests`, clone the `CancellationToken` for the given `request_id`, drop lock.
3. If found: call `token.cancel()`, return `Ok(())`.
4. If not found: return `Err(ChatError::RequestNotFound(request_id))`.

**Tests (add `#[cfg(test)]` module):**

- `chat_error_serializes_as_screaming_snake_case` — serialize `ChatError::CredentialError("test".into())` to JSON, assert contains `"CREDENTIAL_ERROR"`.
- `chat_error_unauthorized_window_serializes_correctly` — serialize `ChatError::UnauthorizedWindow("bad".into())`, assert contains `"UNAUTHORIZED_WINDOW"`.
- `chat_event_ack_serializes_with_type_field` — serialize `ChatEvent::Ack { request_id: "r1".into() }`, assert JSON contains `"type":"Ack"` and `"request_id":"r1"`.
- `chat_event_delta_serializes_with_type_field` — serialize `ChatEvent::Delta { text: "hello".into() }`, assert `"type":"Delta"` and `"text":"hello"`.
- `chat_event_done_serializes_with_type_field` — serialize `ChatEvent::Done { usage: None, model: "m".into() }`, assert `"type":"Done"`.
- `chat_event_error_serializes_with_type_field` — serialize `ChatEvent::Error { code: "CANCELLED".into(), message: "...".into() }`, assert `"type":"Error"` and `"code":"CANCELLED"`.
- `chat_send_has_no_api_key_parameter` — Document this as a compile-time invariant enforced by the type system. Add a comment in the test module: "D-10 invariant verified: chat_send signature does not include api_key. Enforced by type system. `cargo check` is the authoritative gate."

**Verification:**

```
cargo test -p desktop-ai-client-lib ipc::chat
cargo check -p desktop-ai-client-lib
```

**Done:** Seven `ipc::chat` tests pass; `cargo check` passes with no `api_key` parameter in `chat_send`; `chat_cancel` compiles with `CancellationToken` pattern.

**Depends on:** T-1 (AppState extension, secrecy), T-2 (SseEvent types), T-3 (ProviderMessage, stream_completion), T-4 (routing helpers)

---

### Wave 4: IPC Registration

#### T-6 — Register chat commands in main.rs and capabilities

**Files:**

- `src-tauri/src/main.rs`
- `src-tauri/capabilities/main.json`

**Description:**

Register the two new commands so the Tauri runtime can dispatch them from the renderer.

In `src-tauri/src/main.rs`, add to `tauri::generate_handler![...]`:

- `ipc::chat::chat_send`
- `ipc::chat::chat_cancel`

No other changes to `main.rs` are needed — `AppState::default()` already initializes `active_requests` and `secrets` from T-1; the existing `.manage(AppState::default())` call handles it.

In `src-tauri/capabilities/main.json`, add to the `"permissions"` array:

- `"allow-chat-send"`
- `"allow-chat-cancel"`

These follow the same `"allow-<command-kebab>"` pattern as the existing `"allow-get-active-surface"` and `"allow-set-active-surface"` entries. Per RESEARCH.md Assumption A1, verify the exact key format against the first `cargo tauri dev` error output — the error message will state the expected capability key if the format differs.

After editing, run `cargo check` to confirm the handler macro resolves. The capability key format cannot be verified until a Tauri build is attempted (requires full toolchain); document this as a known verification gap requiring a live `cargo tauri dev` run.

**Verification:**

```
cargo check -p desktop-ai-client-lib
```

**Done:** `cargo check` passes with `chat_send` and `chat_cancel` in `generate_handler!`; capabilities JSON updated.

**Depends on:** T-5 (ipc::chat module must exist and compile)

---

### Wave 5: Frontend API + Store (parallel)

#### T-7 — Implement src/lib/api/chat.ts TypeScript API wrapper

**Files:**

- `src/lib/api/chat.ts`

**Description:**

Create the TypeScript thin wrapper over `invoke` and `Channel`. This file is the only place in the frontend that imports `@tauri-apps/api/core`. All other frontend modules import from here.

Define the `ChatEvent` discriminated union (per D-02 and RESEARCH.md Section 1, TypeScript block):

```
export type ChatEvent =
  | { type: 'Ack';   request_id: string }
  | { type: 'Delta'; text: string }
  | { type: 'Done';  usage?: { prompt_tokens: number; completion_tokens: number }; model: string }
  | { type: 'Error'; code: string; message: string };
```

Define `ChatMessage`: `export type ChatMessage = { role: 'user' | 'assistant'; content: string }`.

Define `ChatSendParams`:

```
export type ChatSendParams = {
  messages: ChatMessage[];
  model?: string;
  conversationId?: string;
  maxCompletionTokens?: number;
  temperature?: number;
  onEvent: (event: ChatEvent) => void;
};
```

Implement `export async function chatSend(params: ChatSendParams): Promise<void>`:

- Creates `const channel = new Channel<ChatEvent>()`.
- Sets `channel.onmessage = params.onEvent`.
- Calls `invoke('chat_send', { messages: params.messages, model: params.model ?? null, conversationId: params.conversationId ?? null, maxCompletionTokens: params.maxCompletionTokens ?? null, temperature: params.temperature ?? null, channel })`.
- Note: Tauri converts camelCase JS param names to snake_case Rust param names automatically via the IPC layer. Verify this matches the Rust `chat_send` signature field names.

Implement `export async function chatCancel(requestId: string): Promise<void>`:

- Calls `invoke('chat_cancel', { requestId })`.

**Verification:**

```
npx tsc --noEmit
```

(If TypeScript strict mode is configured, all types must resolve without errors.)

**Done:** `chat.ts` compiles; `ChatEvent` type is exported; `chatSend` and `chatCancel` are exported functions.

**Depends on:** (none — pure TypeScript; no runtime Rust dependency for type-checking)

---

#### T-8 — Implement src/lib/stores/chat.ts Svelte 5 chat store

**Files:**

- `src/lib/stores/chat.ts`

**Description:**

Create the reactive chat store using Svelte 5 runes. Follow the `createSurfaceStore()` pattern from `src/lib/stores/surface.ts` exactly: factory function returning an object with getter-only reactive properties.

Define `ChatMessageState`:

```
export type ChatMessageState = {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  streaming: boolean;
  status: 'complete' | 'incomplete' | 'error';
};
```

Implement `createChatStore()` factory function using `$state` runes:

- `let messages = $state<ChatMessageState[]>([])` — full message list.
- `let streamingId = $state<string | null>(null)` — ID of the currently-streaming assistant message; null when idle.
- `let requestId = $state<string | null>(null)` — backend `request_id` received via `ChatEvent::Ack`; held so the cancel button can call `chatCancel`.
- `let loading = $state(false)` — true from submit until first Delta or terminal event.
- `let error = $state<string | null>(null)`.

Implement `async function sendMessage(content: string): Promise<void>`:

- Appends user message to `messages` with a generated local `id` (e.g., `crypto.randomUUID()`).
- Sets `loading = true`, `error = null`.
- Inserts a placeholder assistant message with `streaming: false, status: 'complete', content: ''` — this is the "thinking" placeholder per D-05.
- Sets `streamingId` to the placeholder's id.
- Calls `chatSend({ messages: ..., onEvent: handleEvent })`.
- On `invoke` rejection: sets `error = normalizeIpcError(e)`, removes placeholder, resets `loading`.

Implement private `function handleEvent(event: ChatEvent): void`:

- `Ack`: set `requestId = event.request_id`. (Store holds this for cancel.)
- `Delta`: transition placeholder to streaming bubble if this is the first delta (set `streaming: true`, update `loading = false` per D-05). Append `event.text` to the streaming message's content.
- `Done`: mark streaming message as `streaming: false, status: 'complete'`. Clear `streamingId`, `requestId`. Update model display if needed.
- `Error` with `code === 'CANCELLED'`: mark streaming message as `streaming: false, status: 'incomplete'` — per D-06. The UI reads `status: 'incomplete'` to render the amber badge. Clear `streamingId`, `requestId`.
- `Error` (other): set `error = normalizeIpcError(event)`, mark streaming message `status: 'error'`. Clear `streamingId`.

Implement `async function cancelRequest(): Promise<void>`:

- If `requestId` is null, no-op.
- Calls `chatCancel(requestId)`.
- On rejection: log warning (cancel is best-effort; channel will deliver `CANCELLED` event regardless).

Reuse `normalizeIpcError` — import it from `$lib/stores/surface.ts` (it is already exported there as a module-internal function — if not exported, copy the implementation into `chat.ts` as a local private function; do not create a third copy).

Return object with getters:

- `get messages()`, `get streamingId()`, `get requestId()`, `get loading()`, `get error()`
- `get canCancel()` — `$derived(requestId !== null)`
- `sendMessage`, `cancelRequest`

Export as singleton: `export const chatStore = createChatStore()`.

**Verification:**

```
npx tsc --noEmit
```

**Done:** TypeScript compiles; `chatStore.sendMessage` and `chatStore.cancelRequest` are callable; `D-03` (terminal state via channel only) is enforced in `handleEvent`.

**Depends on:** T-7 (imports `chatSend`, `chatCancel`, `ChatEvent` from `$lib/api/chat.ts`)

---

### Wave 6: UI Components + ChatSurface Wiring (parallel)

#### T-9 — Implement ChatInput, ChatMessage, StreamingBubble Svelte components

**Files:**

- `src/lib/components/chat/ChatInput.svelte`
- `src/lib/components/chat/ChatMessage.svelte`
- `src/lib/components/chat/StreamingBubble.svelte`

**Description:**

Create the `src/lib/components/chat/` directory and three components. All use Svelte 5 runes syntax (`$props()`, `$derived`, `$state`). Follow the established color/sizing conventions from `ChatSurface.svelte` (dark background `#1a1a1a`, text `#e0e0e0`, border `#2a2a2a`).

**ChatInput.svelte:**

- Props (via `$props()`): `onsubmit: (text: string) => void`, `disabled: boolean`, `showCancel: boolean`, `oncancel: () => void`.
- Renders a `<form>` with a `<textarea>` and submit button. On `Enter` (without Shift) in textarea: submit if non-empty. On Shift+Enter: newline.
- When `disabled`: textarea is `disabled` attribute, submit button shows spinner or "Sending..." label.
- When `showCancel`: renders a secondary "Cancel" button calling `oncancel()`. The cancel button is anchored in the input area, not inside the message bubble (per D-05).
- Accessibility: `aria-label` on textarea (`"Message input"`), submit button type `"submit"`, cancel button type `"button"`.

**ChatMessage.svelte:**

- Props: `role: 'user' | 'assistant'`, `content: string`, `status: 'complete' | 'incomplete' | 'error'`, `streaming: boolean`.
- Renders a message bubble. User messages: right-aligned, accent background. Assistant messages: left-aligned, card background.
- When `status === 'incomplete'` (per D-06): append an amber-colored badge element after the content — `<span class="cancelled-badge">(Cancelled)</span>`. Apply `class="dimmed"` to the content text element to reduce opacity to ~0.6.
- When `streaming === true`: append a blinking cursor element (`<span class="cursor" aria-hidden="true">|</span>`) after the content.
- No streaming animation logic here — `StreamingBubble` handles the transitional state.

**StreamingBubble.svelte:**

- Props: `content: string`, `loading: boolean`.
- When `loading === true` (placeholder state per D-05): renders a skeleton/thinking indicator (e.g., three animated dots or a "Thinking..." text with `aria-live="polite"`).
- When `loading === false` and `content` is accumulating: renders `content` directly with a blinking cursor.
- Transitions from skeleton to content on first non-empty `content` — use a Svelte `{#if}` on `content.length > 0`.
- Accessibility: `aria-live="polite"` on the content container so screen readers announce updates without interruption.

**Verification:**

```
npx tsc --noEmit
```

(All three components must compile. Visual verification requires human review in Wave 7 integration step.)

**Done:** Three components in `src/lib/components/chat/` compile without TypeScript errors; `status: 'incomplete'` renders amber badge per D-06; `loading` prop controls skeleton vs. streaming display per D-05.

**Depends on:** T-8 (ChatMessage type, status values from store shape inform prop types)

---

#### T-10 — Replace ChatSurface placeholder with live chat UI

**Files:**

- `src/lib/components/surfaces/ChatSurface.svelte`

**Description:**

Replace the placeholder content in `ChatSurface.svelte` with a working chat surface that wires the store to the new components. The existing `<div class="surface chat-surface">` shell and styles remain; replace only the `<div class="surface-body">` contents.

Import `chatStore` from `$lib/stores/chat.ts`. Import `ChatInput`, `ChatMessage`, `StreamingBubble` from their paths under `$lib/components/chat/`.

Layout inside `surface-body`:

1. **Message list** — a scrollable `<div class="message-list" aria-live="polite" aria-label="Conversation">`. Maps `chatStore.messages` with `{#each chatStore.messages as msg (msg.id)}`. For the message with `id === chatStore.streamingId` (currently streaming): render `<StreamingBubble content={msg.content} loading={chatStore.loading} />`. For all other messages: render `<ChatMessage role={msg.role} content={msg.content} status={msg.status} streaming={msg.streaming} />`.
2. **Error display** — `{#if chatStore.error}<p class="chat-error" role="alert">{chatStore.error}</p>{/if}`.
3. **ChatInput** — `<ChatInput onsubmit={handleSubmit} disabled={chatStore.loading} showCancel={chatStore.canCancel} oncancel={chatStore.cancelRequest} />`.

Implement `function handleSubmit(text: string)` — calls `chatStore.sendMessage(text)`.

Auto-scroll: derive a scroll container ref; on `chatStore.messages` length change, scroll to bottom with `el.scrollTop = el.scrollHeight`. Use `$effect` rune.

Add CSS for `.message-list` (flex column, overflow-y auto, flex: 1 1 auto) and `.chat-error` (amber text, role alert). Match color palette from existing surface styles.

**Verification:**

```
npx tsc --noEmit
```

**Done:** `ChatSurface.svelte` compiles; imports resolve; `chatStore` is wired to all three components; error and cancel states are handled.

**Depends on:** T-8 (chatStore), T-9 (chat components)

---

### Wave 7: Integration Verification

#### T-11 — Full compile + test suite verification

**Files:** (read-only verification — no file modifications unless a compile error requires a fix)

**Description:**

Run the full verification sequence to confirm the phase is internally consistent. This task produces no new files; it validates that all prior tasks compose correctly.

Step 1 — Rust unit tests:

```
cargo test -p desktop-ai-client-lib
```

Expected passing tests:

- `security::secrets::get_credential_status_returns_missing_when_no_key`
- `security::secrets::get_credential_status_returns_configured_when_key_present`
- `security::secrets::get_provider_key_returns_not_configured_when_missing`
- `app_state::tests::app_state_initializes_active_requests_empty`
- `providers::sse::parse_sse_line_extracts_delta_content`
- `providers::sse::parse_sse_line_ignores_comment_lines`
- `providers::sse::parse_sse_line_handles_done_sentinel`
- `providers::sse::parse_sse_line_returns_none_for_empty`
- `providers::sse::parse_sse_line_detects_mid_stream_error`
- `providers::sse::parse_sse_line_skips_delta_with_empty_content`
- `providers::openrouter::default_model_is_correct`
- `providers::openrouter::provider_message_serializes_role_and_content`
- `providers::openrouter::chat_completion_request_sets_stream_true`
- `providers::routing::build_provider_messages_prepends_system_prompt`
- `providers::routing::build_provider_messages_preserves_order`
- `providers::routing::select_model_returns_default_when_none`
- `providers::routing::select_model_returns_requested_when_some`
- `ipc::chat::chat_error_serializes_as_screaming_snake_case`
- `ipc::chat::chat_error_unauthorized_window_serializes_correctly`
- `ipc::chat::chat_event_ack_serializes_with_type_field`
- `ipc::chat::chat_event_delta_serializes_with_type_field`
- `ipc::chat::chat_event_done_serializes_with_type_field`
- `ipc::chat::chat_event_error_serializes_with_type_field`

Step 2 — D-10 invariant check (no `api_key` in IPC surface):

```
cargo check -p desktop-ai-client-lib
```

Verify by inspection: `grep -n "api_key" src-tauri/src/ipc/chat.rs` must return no matches in function signatures or struct fields.

Step 3 — Frontend TypeScript compile:

```
npx tsc --noEmit
```

Step 4 — Regression: Phase 1 tests must still pass:

```
cargo test -p desktop-ai-client-lib app_state::tests::surface_round_trips_through_string
cargo test -p desktop-ai-client-lib ipc::app_shell::tests::shell_error_serializes_with_code_field
```

If any test fails: fix the root cause in the relevant task's file before marking T-11 complete. Do not add `#[ignore]` attributes.

**Verification:**

```
cargo test -p desktop-ai-client-lib
npx tsc --noEmit
```

**Done:** All 23 Rust unit tests pass; `npx tsc --noEmit` exits 0; `grep api_key src-tauri/src/ipc/chat.rs` finds no IPC parameter; Phase 1 regression tests still pass.

**Depends on:** T-1, T-2, T-3, T-4, T-5, T-6, T-7, T-8, T-9, T-10

---

## Source Audit

| Source                                            | Item                                                                         | Covered By                   | Status  |
| ------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------- | ------- |
| GOAL                                              | Route prompts through deterministic provider selection                       | T-4, T-5                     | COVERED |
| GOAL                                              | Robust streaming transport                                                   | T-2, T-3, T-5                | COVERED |
| REQ ROUTE-01                                      | Prompt routed through deterministic provider selection                       | T-3, T-4, T-5                | COVERED |
| REQ ROUTE-02                                      | Streamed output in order, partial output, cancellation, typed error handling | T-2, T-3, T-5, T-8, T-9      | COVERED |
| D-01 `Channel<ChatEvent>`                         | T-5 (chat_send signature)                                                    | COVERED                      |
| D-02 ChatEvent variants (Ack, Delta, Done, Error) | T-5 (enum definition)                                                        | COVERED                      |
| D-03 terminal state via channel only              | T-5 (run_stream), T-8 (handleEvent)                                          | COVERED                      |
| D-04 `chat_cancel` + CANCELLED                    | T-5 (chat_cancel), T-8 (handleEvent)                                         | COVERED                      |
| D-05 hybrid loading UX                            | T-9 (StreamingBubble), T-10 (ChatSurface)                                    | COVERED                      |
| D-06 cancel amber badge + dim + incomplete        | T-9 (ChatMessage incomplete status)                                          | COVERED                      |
| D-07 thin secrets stub                            | T-1 (security::secrets)                                                      | COVERED                      |
| D-08 `get_provider_key` + `get_credential_status` | T-1 (secrets.rs functions)                                                   | COVERED                      |
| D-09 env-var backing in AppState                  | T-1 (SecretsState::default)                                                  | COVERED                      |
| D-10 no api_key IPC param                         | T-5 (signature, test comment, grep gate in T-11)                             | COVERED                      |
| D-11 chat_send parameter shape                    | T-5 (signature)                                                              | COVERED                      |
| D-12 system prompt backend-owned                  | T-4 (routing prepend), T-5 (not in IPC)                                      | COVERED                      |
| D-13 default model constant                       | T-3 (DEFAULT_MODEL), T-4 (select_model)                                      | COVERED                      |
| D-14 request_id via Ack before spawn              | T-5 (channel.send Ack before tokio::spawn)                                   | COVERED                      |
| RESEARCH SSE line parser                          | T-2                                                                          | COVERED                      |
| RESEARCH openrouter HTTP adapter                  | T-3                                                                          | COVERED                      |
| RESEARCH CancellationToken pattern                | T-5                                                                          | COVERED                      |
| RESEARCH SecretString spawn pattern               | T-1, T-5                                                                     | COVERED                      |
| RESEARCH Cargo deps                               | T-1                                                                          | COVERED                      |
| RESEARCH capabilities JSON                        | T-6                                                                          | COVERED                      |
| RESEARCH frontend Channel TypeScript              | T-7                                                                          | COVERED                      |
| Deferred: settings UI model picker                | —                                                                            | EXCLUDED (Deferred)          |
| Deferred: capability-based model selection        | —                                                                            | EXCLUDED (Deferred)          |
| Deferred: Stronghold keychain                     | —                                                                            | EXCLUDED (Deferred, Phase 4) |
| Deferred: conversation persistence write          | —                                                                            | EXCLUDED (Deferred, Phase 3) |

---

## Output

When execution is complete, create `.planning/phases/02-routing/02-SUMMARY.md` documenting:

- Which files were created or modified
- Key implementation decisions made (especially the AppState Arc/AppHandle pattern used for spawn)
- Actual capability key format confirmed at build time (resolve Assumption A1)
- Any deviation from this plan and rationale
