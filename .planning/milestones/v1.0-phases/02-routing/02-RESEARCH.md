# Phase 2: Routing — Research

**Researched:** 2026-06-14
**Domain:** Tauri v2 IPC channels, reqwest SSE streaming, tokio cancellation, secrecy crate, OpenRouter API
**Confidence:** HIGH (all crates verified on crates.io; Tauri Channel API verified against official v2 docs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Use Tauri v2 `Channel<ChatEvent>` (not Tauri events, not polling) as the streaming transport.
- **D-02:** `ChatEvent` is a tagged enum with four variants:
  - `ChatEvent::Delta { text: String }`
  - `ChatEvent::Done { usage: Option<TokenUsage>, model: String }`
  - `ChatEvent::Error { code: String, message: String }`
  - `ChatEvent::Ack { request_id: String }` — sent before streaming begins so the frontend can call `chat_cancel`
- **D-03:** `chat_send` signals all terminal state through the channel only. No dual data paths.
- **D-04:** Cancellation via separate `chat_cancel` IPC command taking `{ request_id }`. Rust aborts SSE and emits `ChatEvent::Error { code: "CANCELLED", ... }`.
- **D-05:** Hybrid loading UX: placeholder on submit, transition to streaming bubble on first Delta.
- **D-06:** On cancel: freeze streaming, append amber badge, dim text, persist with `status: "incomplete"`.
- **D-07:** Implement thin `security::secrets` stub now. Phase 4 replaces internals.
- **D-08:** Stub exposes `get_provider_key(provider)` and `get_credential_status(provider)` only.
- **D-09:** Phase 2 backing store: read `OPENROUTER_API_KEY` from env at startup; hold in `AppState` behind `Mutex`.
- **D-10 (hard invariant):** IPC commands MUST NEVER accept an `api_key` parameter.
- **D-11:** `chat_send` parameters: `messages`, `model?`, `conversation_id?`, `max_completion_tokens?`, `temperature?`.
- **D-12:** System prompt is backend-owned. Never accepted from `chat_send`.
- **D-13:** Default model: backend-configured constant (e.g. `anthropic/claude-sonnet-4-6`).
- **D-14:** Backend generates `request_id` per `chat_send`. Delivered to frontend via `ChatEvent::Ack { request_id }` before streaming.

### Claude's Discretion

- Exact Rust crate for `SecretString`
- Exact HTTP client for OpenRouter SSE
- Window-label enforcement pattern for `chat_send`
- Error enum naming for `ChatError` variants

### Deferred Ideas (OUT OF SCOPE)

- Settings UI model picker
- Capability-based model selection
- Full Stronghold/OS keychain secrets store (Phase 4)
- Conversation persistence write (Phase 3; `conversation_id` field is wired but no DB write)
</user_constraints>

---

## Summary

This phase wires end-to-end streaming from a Svelte 5 frontend through the Tauri IPC boundary to the OpenRouter `/api/v1/chat/completions` endpoint and back. The three moving parts are: (1) `tauri::ipc::Channel<ChatEvent>` as the streaming transport — a per-invocation typed pipe that the frontend creates and passes to `invoke`; (2) `reqwest` with the `stream` feature consuming OpenRouter's SSE response using `bytes_stream()` + `futures_util::StreamExt`; and (3) `tokio_util::sync::CancellationToken` stored per request in `AppState`, cancellable from a separate `chat_cancel` IPC command.

The credential model is a thin stub: `security::secrets` reads `OPENROUTER_API_KEY` from the environment at startup, wraps it in `secrecy::SecretString`, holds it behind a `Mutex` in `AppState`, and never crosses the IPC boundary. `chat_send` retrieves it internally.

The dependency chain is: `ipc::chat` → `providers::routing` → `providers::openrouter` → `providers::sse` + `security::secrets`. Backend modules must not import from `ipc::`.

**Primary recommendation:** Implement `chat_send` with a `Channel<ChatEvent>` parameter; Rust spawns a task that sends `Ack` immediately, then streams deltas from OpenRouter SSE, then sends `Done` or `Error`. Cancel token is stored in `AppState` before the task starts.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Prompt submission | API/Backend (`ipc::chat`) | Frontend (invoke caller) | Backend validates, enforces secrets invariant |
| SSE streaming to provider | API/Backend (`providers::sse`) | — | Credentials must not cross IPC; streaming happens entirely in Rust |
| Token delivery to renderer | IPC Channel (`tauri::ipc::Channel`) | — | Per-invocation typed pipe; not global events |
| Cancellation signal | API/Backend (`ipc::chat::chat_cancel`) | Frontend (button calls invoke) | Backend owns the CancellationToken registry |
| Credential storage | API/Backend (`security::secrets`) | AppState Mutex | Never touches renderer |
| Request ID generation | API/Backend | — | Generated in `chat_send` before spawning task |
| Streaming UI state | Frontend (`src/lib/stores/`) | — | Reactive state from channel messages only |

---

## 1. Tauri v2 Channel Streaming

### How Channel<T> Works

`tauri::ipc::Channel<T>` is the standard mechanism for streaming data from a Rust command back to the frontend. The frontend allocates the channel, passes it as an invoke argument; Rust receives it as a typed command parameter and calls `.send(value)` to push events.

`[VERIFIED: docs.rs/tauri/latest/tauri/ipc/struct.Channel.html]`
`[VERIFIED: v2.tauri.app/develop/calling-frontend/]`

### TypeScript — Create and Pass Channel

```typescript
// src/lib/api/chat.ts
import { Channel, invoke } from '@tauri-apps/api/core';

// Define the exact shape of ChatEvent variants (must match Rust serde output)
type ChatEvent =
  | { type: 'Ack';   request_id: string }
  | { type: 'Delta'; text: string }
  | { type: 'Done';  usage?: { prompt_tokens: number; completion_tokens: number }; model: string }
  | { type: 'Error'; code: string; message: string };

export async function chatSend(params: {
  messages: { role: string; content: string }[];
  model?: string;
  conversation_id?: string;
  max_completion_tokens?: number;
  temperature?: number;
  onEvent: (event: ChatEvent) => void;
}): Promise<void> {
  const channel = new Channel<ChatEvent>();
  channel.onmessage = params.onEvent;

  await invoke('chat_send', {
    messages: params.messages,
    model: params.model ?? null,
    conversationId: params.conversation_id ?? null,
    maxCompletionTokens: params.max_completion_tokens ?? null,
    temperature: params.temperature ?? null,
    channel,   // Tauri serializes this as its internal channel ID
  });
}

export async function chatCancel(requestId: string): Promise<void> {
  await invoke('chat_cancel', { requestId });
}
```

**Key detail:** The channel instance is passed as a named parameter (not positionally). Tauri's IPC layer serializes it by its internal numeric ID.

### Rust — Receive Channel as Command Parameter

```rust
// src-tauri/src/ipc/chat.rs
use tauri::ipc::Channel;

#[tauri::command]
pub async fn chat_send(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    messages: Vec<ChatMessage>,
    model: Option<String>,
    conversation_id: Option<String>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    channel: Channel<ChatEvent>,   // <-- Tauri injects this automatically
) -> Result<(), ChatError> {
    assert_main_window(&window)?;
    // ...
}
```

### Rust — Send Events Through the Channel

```rust
// channel.send() requires ChatEvent: serde::Serialize
channel.send(ChatEvent::Ack { request_id: request_id.clone() })
    .map_err(|e| ChatError::ChannelError(e.to_string()))?;

channel.send(ChatEvent::Delta { text: token })
    .map_err(|e| ChatError::ChannelError(e.to_string()))?;

channel.send(ChatEvent::Done { usage: Some(usage), model: resolved_model })
    .map_err(|e| ChatError::ChannelError(e.to_string()))?;
```

`Channel::send(&self, data: T) -> Result<()>` requires `T: IpcResponse`.
`serde::Serialize` satisfies `IpcResponse`. `[VERIFIED: docs.rs/tauri/latest]`

### ChatEvent Rust Definition

```rust
// src-tauri/src/ipc/chat.rs

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum ChatEvent {
    Ack { request_id: String },
    Delta { text: String },
    Done {
        usage: Option<TokenUsage>,
        model: String,
    },
    Error { code: String, message: String },
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}
```

**Note on serde tag:** `#[serde(tag = "type", rename_all = "PascalCase")]` produces `{ "type": "Delta", "text": "..." }` which the TypeScript discriminated union above matches directly. Alternatively use `rename_all = "snake_case"` if preferred — just keep Rust and TS in sync.

### ChatError Definition (follow ShellError pattern)

```rust
// src-tauri/src/ipc/chat.rs

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChatError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("credential error: {0}")]
    CredentialError(String),
    #[error("provider error: {0}")]
    ProviderError(String),
    #[error("channel error: {0}")]
    ChannelError(String),
    #[error("request not found: {0}")]
    RequestNotFound(String),
}
```

Matches the established `{ code: "SCREAMING_SNAKE_CASE", message: string }` IPC error shape. `[VERIFIED: src-tauri/src/ipc/app_shell.rs]`

---

## 2. reqwest SSE Streaming

### Required Cargo Features

```toml
reqwest = { version = "0.13", features = ["json", "stream"] }
futures-util = "0.3"
```

The `stream` feature enables `Response::bytes_stream()`. `futures-util` provides `StreamExt::next()`.
`[VERIFIED: docs.rs/reqwest/latest/reqwest/struct.Response.html]`
`[VERIFIED: crates.io — reqwest 0.13.4, 527M downloads; futures-util 0.3.32, 676M downloads]`

### OpenRouter Request

```rust
// src-tauri/src/providers/openrouter.rs
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};

const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1";
const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4-6";

#[derive(serde::Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: &'a [ProviderMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

pub async fn stream_completion(
    client: &Client,
    api_key: &SecretString,
    model: &str,
    messages: &[ProviderMessage],
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<reqwest::Response, ChatError> {
    let body = ChatCompletionRequest {
        model,
        messages,
        stream: true,
        max_completion_tokens,
        temperature,
    };

    let response = client
        .post(format!("{OPENROUTER_BASE}/chat/completions"))
        .header("Authorization", format!("Bearer {}", api_key.expose_secret()))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://desktop-ai-client")  // OpenRouter recommends this
        .json(&body)
        .send()
        .await
        .map_err(|e| ChatError::ProviderError(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        return Err(ChatError::ProviderError(format!("HTTP {status}")));
    }

    Ok(response)
}
```

### SSE Line Parsing

```rust
// src-tauri/src/providers/sse.rs
use futures_util::StreamExt;

/// Parsed SSE chunk from OpenRouter streaming response.
#[derive(Debug)]
pub enum SseEvent {
    Delta { text: String },
    Done { usage: Option<SseUsage>, model: String },
    Comment,
    Unknown,
}

#[derive(Debug, serde::Deserialize)]
struct SseChunk {
    id: Option<String>,
    model: Option<String>,
    choices: Vec<SseChoice>,
    usage: Option<SseUsage>,
}

#[derive(Debug, serde::Deserialize)]
struct SseChoice {
    delta: Option<SseDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SseDelta {
    content: Option<String>,
    role: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SseUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Drives the SSE stream, emitting parsed events through a callback.
/// Calls `on_event` for each Delta/Done/error; returns when stream ends or is cancelled.
pub async fn drive_sse_stream<F>(
    response: reqwest::Response,
    mut on_event: F,
) -> Result<(), String>
where
    F: FnMut(SseEvent) -> Result<(), String>,
{
    let mut stream = response.bytes_stream();
    let mut line_buf = String::new();
    let mut final_model: Option<String> = None;
    let mut final_usage: Option<SseUsage> = None;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&chunk);

        line_buf.push_str(&text);

        // Process complete lines from buffer
        while let Some(newline_pos) = line_buf.find('\n') {
            let line = line_buf[..newline_pos].trim_end_matches('\r').to_string();
            line_buf.drain(..=newline_pos);

            // SSE comment — ignore (OpenRouter sends ": OPENROUTER PROCESSING")
            if line.starts_with(':') || line.is_empty() {
                continue;
            }

            // Only handle "data: ..." lines
            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    on_event(SseEvent::Done {
                        usage: final_usage.clone(),
                        model: final_model.clone().unwrap_or_default(),
                    })?;
                    return Ok(());
                }

                match serde_json::from_str::<SseChunk>(data) {
                    Ok(chunk) => {
                        if let Some(m) = &chunk.model {
                            final_model = Some(m.clone());
                        }
                        if let Some(u) = chunk.usage {
                            final_usage = Some(u);
                        }
                        for choice in &chunk.choices {
                            if let Some(delta) = &choice.delta {
                                if let Some(content) = &delta.content {
                                    if !content.is_empty() {
                                        on_event(SseEvent::Delta { text: content.clone() })?;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("SSE parse error: {e} on line: {data}");
                        // Non-fatal: skip malformed chunk
                    }
                }
            }
            // Lines without "data: " prefix (event:, id:, retry:) — ignored for now
        }
    }

    Ok(())
}
```

**Important:** `line_buf.find('\n')` handles both `\n` and `\r\n` (trim_end_matches('\r')). OpenRouter also sends SSE comment lines like `: OPENROUTER PROCESSING` — these must be ignored, not parsed as JSON. `[VERIFIED: openrouter.ai/docs/api/reference/streaming]`

---

## 3. CancellationToken — Per-Request Cancellation

### How CancellationToken Works

`tokio_util::sync::CancellationToken` is cloneable. The parent token is stored; a `.clone()` is passed into the spawned task. Calling `.cancel()` on the parent wakes all tasks awaiting `.cancelled()` on any clone. `[VERIFIED: docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html]`

### Cargo Dependency

```toml
tokio-util = { version = "0.7", features = ["rt"] }
```

`[VERIFIED: crates.io — tokio-util 0.7.18, 605M downloads, github.com/tokio-rs/tokio]`

Also add the `sync` feature of tokio (which the current Cargo.toml lacks):

```toml
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
```

### AppState Extension

```rust
// src-tauri/src/app_state.rs — extend existing AppState

use std::collections::HashMap;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    pub shell: Mutex<ShellState>,
    // Phase 2 additions:
    pub active_requests: Mutex<HashMap<String, CancellationToken>>,
    pub secrets: Mutex<SecretsState>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            shell: Mutex::new(ShellState::default()),
            active_requests: Mutex::new(HashMap::new()),
            secrets: Mutex::new(SecretsState::default()),
        }
    }
}

pub struct SecretsState {
    pub openrouter_key: Option<secrecy::SecretString>,
}

impl Default for SecretsState {
    fn default() -> Self {
        let key = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .map(|v| secrecy::SecretString::new(v.into()));
        Self { openrouter_key: key }
    }
}
```

### chat_send — Register Token, Spawn Task

```rust
// src-tauri/src/ipc/chat.rs (key excerpt)

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tauri::command]
pub async fn chat_send(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    messages: Vec<ChatMessage>,
    model: Option<String>,
    conversation_id: Option<String>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    channel: Channel<ChatEvent>,
) -> Result<(), ChatError> {
    assert_main_window(&window)?;

    let request_id = Uuid::new_v4().to_string();
    let token = CancellationToken::new();

    // Register before spawning so chat_cancel can find it immediately
    {
        let mut requests = state.active_requests.lock()
            .map_err(|e| ChatError::ProviderError(format!("lock poisoned: {e}")))?;
        requests.insert(request_id.clone(), token.clone());
    }

    // Retrieve API key before spawning — don't hold lock across await
    let api_key = {
        let secrets = state.secrets.lock()
            .map_err(|e| ChatError::CredentialError(format!("lock poisoned: {e}")))?;
        secrets.openrouter_key.clone()
            .ok_or_else(|| ChatError::CredentialError("OPENROUTER_API_KEY not configured".into()))?
    };

    let resolved_model = model.unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let state_clone = state.inner().clone();  // or Arc clone depending on state setup
    let request_id_clone = request_id.clone();

    // Send Ack immediately so frontend has request_id for cancellation
    channel.send(ChatEvent::Ack { request_id: request_id.clone() })
        .map_err(|e| ChatError::ChannelError(e.to_string()))?;

    // Spawn the streaming task
    let token_clone = token.clone();
    tokio::spawn(async move {
        let result = run_stream(
            &api_key,
            &resolved_model,
            messages,
            max_completion_tokens,
            temperature,
            &channel,
            token_clone,
        ).await;

        if let Err(e) = result {
            let _ = channel.send(ChatEvent::Error {
                code: "PROVIDER_ERROR".into(),
                message: e,
            });
        }

        // Clean up registry
        if let Ok(mut requests) = state_clone.active_requests.lock() {
            requests.remove(&request_id_clone);
        }
    });

    Ok(())
}
```

### chat_cancel Command

```rust
#[tauri::command]
pub async fn chat_cancel(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    request_id: String,
) -> Result<(), ChatError> {
    assert_main_window(&window)?;

    let token = {
        let requests = state.active_requests.lock()
            .map_err(|e| ChatError::ProviderError(format!("lock: {e}")))?;
        requests.get(&request_id).cloned()
    };

    match token {
        Some(t) => {
            t.cancel();
            Ok(())
        }
        None => Err(ChatError::RequestNotFound(request_id)),
    }
}
```

### Streaming Task with Cancellation Select

```rust
async fn run_stream(
    api_key: &secrecy::SecretString,
    model: &str,
    messages: Vec<ChatMessage>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    channel: &Channel<ChatEvent>,
    cancel_token: CancellationToken,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let provider_msgs: Vec<ProviderMessage> = messages.into_iter()
        .map(|m| ProviderMessage { role: m.role, content: m.content })
        .collect();

    let response = tokio::select! {
        r = openrouter::stream_completion(&client, api_key, model, &provider_msgs, max_completion_tokens, temperature) => r?,
        _ = cancel_token.cancelled() => {
            let _ = channel.send(ChatEvent::Error {
                code: "CANCELLED".into(),
                message: "Request cancelled by user".into(),
            });
            return Ok(());
        }
    };

    let channel_ref = channel.clone();
    let cancel_ref = cancel_token.clone();

    // Drive SSE with cancellation interleaved per-chunk
    let mut stream = response.bytes_stream();
    let mut line_buf = String::new();
    let mut final_model: Option<String> = Some(model.to_string());
    let mut final_usage: Option<sse::SseUsage> = None;

    loop {
        let chunk_result = tokio::select! {
            chunk = stream.next() => chunk,
            _ = cancel_ref.cancelled() => {
                let _ = channel_ref.send(ChatEvent::Error {
                    code: "CANCELLED".into(),
                    message: "Request cancelled by user".into(),
                });
                return Ok(());
            }
        };

        match chunk_result {
            None => break,
            Some(Err(e)) => return Err(e.to_string()),
            Some(Ok(bytes)) => {
                let text = String::from_utf8_lossy(&bytes);
                line_buf.push_str(&text);

                while let Some(pos) = line_buf.find('\n') {
                    let line = line_buf[..pos].trim_end_matches('\r').to_string();
                    line_buf.drain(..=pos);

                    if line.starts_with(':') || line.is_empty() { continue; }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data.trim() == "[DONE]" {
                            let _ = channel_ref.send(ChatEvent::Done {
                                usage: final_usage.clone().map(|u| TokenUsage {
                                    prompt_tokens: u.prompt_tokens,
                                    completion_tokens: u.completion_tokens,
                                }),
                                model: final_model.clone().unwrap_or_default(),
                            });
                            return Ok(());
                        }

                        if let Ok(chunk) = serde_json::from_str::<sse::SseChunk>(data) {
                            if let Some(m) = &chunk.model { final_model = Some(m.clone()); }
                            if let Some(u) = chunk.usage { final_usage = Some(u); }
                            for choice in &chunk.choices {
                                if let Some(delta) = &choice.delta {
                                    if let Some(content) = &delta.content {
                                        if !content.is_empty() {
                                            let _ = channel_ref.send(ChatEvent::Delta { text: content.clone() });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
```

---

## 4. secrecy Crate

`[VERIFIED: docs.rs/secrecy/latest; crates.io — secrecy 0.10.3, 119M downloads, github.com/iqlusioninc/crates]`

### Cargo Dependency

```toml
secrecy = { version = "0.10", features = ["alloc"] }
```

The `alloc` feature enables `SecretString`. It is on by default when using the standard library.

### Key Types

```
SecretString   — type alias for SecretBox<String>
SecretBox<T>   — generic wrapper; T must implement Zeroize
ExposeSecret   — trait providing expose_secret() -> &T
```

### Usage Pattern

```rust
use secrecy::{ExposeSecret, SecretString};

// Creating
let key = SecretString::new("sk-or-v1-...".to_string().into());

// Accessing the value (requires ExposeSecret in scope)
let value: &str = key.expose_secret();

// Passing to reqwest
let auth_header = format!("Bearer {}", key.expose_secret());

// Debug output is redacted: SecretString { ... }
// Drop zeroizes the inner String automatically
```

### What SecretString Does NOT Do

- Does NOT implement `serde::Serialize` by default (cannot accidentally serialize to JSON)
- Does NOT implement `Clone` by default (prevents accidental duplication; wrap in `Arc` if sharing across tasks)
- Does NOT print the value in `Debug` or `Display` output

**To share across the spawn boundary:** Either clone the `SecretString` before spawning (if you implement `CloneableSecret` or use a workaround), or read the value before spawn and pass the raw string inside an ephemeral wrapper. The simplest approach: call `.expose_secret()` once, clone the `String`, wrap it in `SecretString::new(cloned.into())` for the spawned task.

```rust
// Shareable pattern for spawned tasks:
let key_for_task = {
    let s = state.secrets.lock().unwrap();
    let raw = s.openrouter_key.as_ref()
        .ok_or(ChatError::CredentialError("no key".into()))?
        .expose_secret()
        .to_string();
    SecretString::new(raw.into())  // New wrapper — drops with the task
};
```

---

## 5. OpenRouter API Shape

`[VERIFIED: openrouter.ai/docs/api/api-reference/chat/send-chat-completion-request]`
`[VERIFIED: openrouter.ai/docs/api/reference/streaming]`

### Base URL

```
https://openrouter.ai/api/v1
```

### Request — POST /chat/completions

```json
{
  "model": "anthropic/claude-sonnet-4-6",
  "messages": [
    { "role": "system", "content": "You are a helpful assistant." },
    { "role": "user",   "content": "Hello" }
  ],
  "stream": true,
  "max_completion_tokens": 2048,
  "temperature": 0.7
}
```

**Headers:**
```
Authorization: Bearer <api_key>
Content-Type: application/json
HTTP-Referer: https://desktop-ai-client   (recommended by OpenRouter for attribution)
```

### SSE Response Format

Each chunk is a `data: <JSON>` line followed by `\n\n`:

```
: OPENROUTER PROCESSING

data: {"id":"gen-abc","object":"chat.completion.chunk","created":1749000000,"model":"anthropic/claude-sonnet-4-6","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

data: {"id":"gen-abc","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}

data: {"id":"gen-abc","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":12,"completion_tokens":7}}

data: [DONE]
```

### Key Parsing Rules

| Line | Action |
|------|--------|
| Starts with `:` | Ignore (SSE comment; OpenRouter sends `: OPENROUTER PROCESSING` to prevent timeout) |
| `data: [DONE]` | Stream complete; emit `ChatEvent::Done` |
| `data: {"choices":[{"delta":{"content":"..."}}]}` | Extract `choices[0].delta.content`; emit `ChatEvent::Delta` |
| `data: {...,"usage":{...}}` | Capture usage for `ChatEvent::Done` (often on the last chunk before `[DONE]`) |
| `data: {"error":{...}}` | Provider error mid-stream (HTTP 200 but error in body); emit `ChatEvent::Error` |
| `event:`, `id:`, `retry:` | Ignore for Phase 2 |

### Mid-Stream Error Format

OpenRouter can signal errors inside a 200 response:

```json
{
  "error": { "message": "...", "code": 429 },
  "choices": [{ "finish_reason": "error" }]
}
```

The SSE parser must check for a top-level `error` key in addition to `choices[0].delta.content`.

---

## 6. Cargo.toml Additions

Current `src-tauri/Cargo.toml` is missing: `reqwest`, `secrecy`, `tokio-util`, `futures-util`. The `tokio` entry also needs the `sync` feature.

```toml
# Add to [dependencies]:
reqwest     = { version = "0.13", features = ["json", "stream"] }
secrecy     = { version = "0.10", features = ["alloc"] }
tokio-util  = { version = "0.7",  features = ["rt"] }
futures-util = "0.3"

# Modify existing tokio entry to add "sync":
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync"] }
```

**Why `sync` on tokio:** `CancellationToken` requires `tokio::sync` internally.

**Version pins:** All are semver-compatible minor pins. Exact versions at research time:
- reqwest 0.13.4 (crates.io, 2026-05-25)
- secrecy 0.10.3 (crates.io)
- tokio-util 0.7.18 (crates.io, 2026-01-04)
- futures-util 0.3.32 (crates.io)

**reqwest TLS:** On Windows, reqwest 0.13 defaults to `rustls-tls`. No additional feature flag needed for HTTPS.

---

## 7. Capabilities JSON Additions

File: `src-tauri/capabilities/main.json`

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-window",
  "description": "Capability set for the main application window.",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "core:app:allow-app-hide",
    "core:window:allow-start-dragging",
    "allow-get-active-surface",
    "allow-set-active-surface",
    "allow-chat-send",
    "allow-chat-cancel"
  ]
}
```

**Format note:** Permission identifiers for custom commands follow the pattern `allow-<command-name-kebab>`. The convention is derived from how Tauri generates capability keys for custom commands. `[ASSUMED — pattern inferred from existing entries; verify with `tauri build` output or Tauri docs for exact generated key names]`

**Also required:** Register the commands in `main.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    ipc::app_shell::get_active_surface,
    ipc::app_shell::set_active_surface,
    ipc::chat::chat_send,      // add
    ipc::chat::chat_cancel,    // add
])
```

---

## 8. Architecture: Module Wiring

```
src-tauri/src/
├── ipc/
│   ├── chat.rs          ← implements chat_send, chat_cancel, ChatError, ChatEvent, ChatMessage
│   └── app_shell.rs     ← existing (pattern reference)
├── providers/
│   ├── mod.rs           ← pub mod routing; pub mod openrouter; pub mod sse;
│   ├── routing.rs       ← select provider, build ProviderMessage vec (thin for Phase 2)
│   ├── openrouter.rs    ← build reqwest request, check HTTP status, return Response
│   └── sse.rs           ← SseChunk/SseDelta structs, bytes_stream parsing, SseEvent enum
├── security/
│   ├── mod.rs           ← pub mod secrets;
│   └── secrets.rs       ← ProviderId enum, SecretsError, get_provider_key(), get_credential_status()
└── app_state.rs         ← extend with active_requests + secrets fields
```

**Dependency direction (enforced, never inverted):**
```
ipc::chat
  → providers::routing
    → providers::openrouter
      → providers::sse
  → security::secrets
  → app_state (CancellationToken registry)
```

---

## Package Legitimacy Audit

> These are Rust (Cargo) crates — slopcheck operates on npm and is not applicable. All crates verified against crates.io directly.

| Package | Registry | Age | Downloads | Source Repo | Disposition |
|---------|----------|-----|-----------|-------------|-------------|
| reqwest 0.13.4 | crates.io | 8+ yrs | 527M | github.com/seanmonstar/reqwest | Approved |
| secrecy 0.10.3 | crates.io | 5+ yrs | 119M | github.com/iqlusioninc/crates | Approved |
| tokio-util 0.7.18 | crates.io | 5+ yrs | 605M | github.com/tokio-rs/tokio | Approved |
| futures-util 0.3.32 | crates.io | 7+ yrs | 676M | github.com/rust-lang/futures-rs | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none (slopcheck is npm-only; Cargo crates verified separately)
**Packages flagged as suspicious:** none

---

## Common Pitfalls

### Pitfall 1: AppState Not Arc-Cloneable for tokio::spawn

**What goes wrong:** `tauri::State<'_, AppState>` cannot be moved into `tokio::spawn` because it has a lifetime tied to the app handle.

**Why it happens:** `tokio::spawn` requires `'static` bounds. `tauri::State` is a reference with a lifetime.

**How to avoid:** Acquire all needed data from state *before* the spawn, then move only owned values (cloned keys, request IDs, `Arc<AppState>`) into the closure. If you need post-spawn access to state (e.g., to remove the token on completion), pass `state.inner()` wrapped in `Arc`.

```rust
// Wrong: state cannot move into spawn
tokio::spawn(async move {
    let _ = state.active_requests.lock(); // compile error
});

// Correct: capture inner Arc before spawn
let state_arc = state.inner().clone(); // AppState must derive Clone or wrap in Arc
tokio::spawn(async move {
    let _ = state_arc.active_requests.lock(); // OK
});
```

**Fix:** Wrap `AppState` in `Arc<AppState>` in `main.rs` and register that. Or use `app.state::<AppState>()` from the `AppHandle` captured before spawn.

### Pitfall 2: Channel.send() After Frontend Drops the Listener

**What goes wrong:** If the user navigates away or closes the channel, `channel.send()` returns an error. Treating this as fatal crashes the Rust task.

**Why it happens:** The channel's JavaScript callback has been garbage-collected.

**How to avoid:** Log and ignore `channel.send()` errors after the first `Done` or `Error` has been sent. The only place send errors matter is before any terminal event.

```rust
// After Done is sent, subsequent errors are expected
let _ = channel.send(ChatEvent::Done { ... }); // ignore result
```

### Pitfall 3: Holding Mutex Lock Across .await

**What goes wrong:** Locking `active_requests` or `secrets` with a `std::sync::Mutex` and then calling `.await` causes a panic or deadlock in async code.

**Why it happens:** `std::sync::Mutex` guards are not `Send` across await points in Tauri's async executor.

**How to avoid:** Always lock, clone or extract the needed value, drop the guard, *then* await.

```rust
// Wrong
let guard = state.secrets.lock().unwrap();
let response = client.send().await; // panic: MutexGuard held across await

// Correct
let api_key = {
    let guard = state.secrets.lock().unwrap();
    guard.openrouter_key.as_ref().unwrap().expose_secret().to_string()
};
// guard dropped here
let response = client.send().await; // safe
```

### Pitfall 4: Missing "stream" Feature on reqwest

**What goes wrong:** `response.bytes_stream()` fails to compile.

**Why it happens:** `bytes_stream()` is gated behind the optional `stream` feature.

**How to avoid:** Add `features = ["json", "stream"]` to the reqwest dependency. `[VERIFIED: docs.rs/reqwest]`

### Pitfall 5: CancellationToken Not Cleaned Up on Normal Completion

**What goes wrong:** The `active_requests` HashMap grows without bound; a stale token can be cancelled by a future `chat_cancel` call with a recycled ID (UUID collision is astronomically unlikely but cleanup is still correct).

**How to avoid:** The spawned task must remove its entry from `active_requests` unconditionally in a `finally`-equivalent block (after both success and error paths).

```rust
tokio::spawn(async move {
    let result = run_stream(...).await;
    // Always clean up, regardless of result
    if let Ok(mut map) = state_arc.active_requests.lock() {
        map.remove(&request_id);
    }
    if let Err(e) = result { ... }
});
```

### Pitfall 6: Capabilities Permission Key Format

**What goes wrong:** Adding `"allow-chat-send"` to capabilities JSON but Tauri generates a different key, resulting in the command being blocked.

**Why it happens:** Tauri's capability key format for custom commands is auto-generated. The exact format should be verified against `tauri build` output or the Tauri docs for the pinned version.

**How to avoid:** After adding the command to `generate_handler!`, run `cargo tauri dev` once and check the error message for the exact expected capability key. `[ASSUMED — verify at build time]`

### Pitfall 7: SecretString Not Clone Without Explicit Implementation

**What goes wrong:** Cannot move `SecretString` into spawned task without explicit clone.

**How to avoid:** See Section 4 "Shareable pattern" — call `.expose_secret()`, clone the `String`, rewrap as `SecretString::new(cloned.into())`. The new wrapper zeroizes independently.

### Pitfall 8: SSE Line Buffering Across Chunks

**What goes wrong:** A chunk from `bytes_stream()` may end in the middle of a line. If you split naively per chunk, you get partial JSON that fails to parse.

**Why it happens:** TCP fragmentation; reqwest yields chunks as they arrive.

**How to avoid:** Maintain `line_buf: String` across loop iterations, appending each chunk's text, and only process complete lines (up to `\n`). The pattern in Section 2 above handles this correctly.

---

## 9. OpenRouter HTTP-Referer Header

OpenRouter's documentation recommends (but does not require) an `HTTP-Referer` header for request attribution. Setting it to a stable identifier like `https://desktop-ai-client` is harmless and may help with OpenRouter's rate-limit attribution.

Do NOT include a `X-Title` header that exposes the user's local machine hostname.

---

## Validation Architecture

`.planning/config.json` — checking nyquist_validation setting.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`, `#[tokio::test]`) |
| Config file | none — cargo test discovers tests in module |
| Quick run command | `cargo test -p desktop-ai-client-lib` |
| Full suite command | `cargo test -p desktop-ai-client-lib -- --include-ignored` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command |
|--------|----------|-----------|-------------------|
| ROUTE-01 | Provider selection routes to OpenRouter | unit | `cargo test providers::routing` |
| ROUTE-01 | Credential retrieved from SecretsState, not IPC param | unit | `cargo test security::secrets` |
| ROUTE-02 | SSE line parser extracts delta content correctly | unit | `cargo test providers::sse` |
| ROUTE-02 | SSE parser ignores comment lines | unit | `cargo test providers::sse::ignores_comment` |
| ROUTE-02 | SSE parser handles [DONE] | unit | `cargo test providers::sse::handles_done` |
| ROUTE-02 | CancellationToken cancels in-flight task | unit | `cargo test ipc::chat::cancel_stops_stream` |
| ROUTE-02 | ChatError serializes as SCREAMING_SNAKE_CASE | unit | `cargo test ipc::chat::error_serialization` |
| D-10 | chat_send has no api_key parameter | compile-time | `cargo check` (type system enforces) |

### Wave 0 Gaps

- `tests/rust/` directory exists but is empty — test files needed for all ROUTE-01/02 coverage above
- No `reqwest` mock in place — unit tests for SSE parser should use canned `&[u8]` inputs, not live network

---

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | partial | Backend-only `SecretString`; env-var stub → Phase 4 keychain |
| V3 Session Management | no | No session tokens; request_id is ephemeral |
| V4 Access Control | yes | `assert_main_window` on all chat commands |
| V5 Input Validation | yes | `ChatMessage` deserialization; reject oversized payloads |
| V6 Cryptography | no | TLS handled by reqwest/rustls; no app-level crypto |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Renderer passes api_key in IPC payload | Info Disclosure | D-10 invariant: never accept api_key parameter |
| Compromised renderer calls chat_send with crafted system prompt | Tampering | System prompt is backend-owned (D-12); not accepted from IPC |
| SSRF via custom endpoint (future) | Elevation of Privilege | Phase 2 has hardcoded OpenRouter URL; custom endpoint validation is Phase 4+ |
| CancellationToken registry never cleaned | DoS | Pitfall 5 above: unconditional cleanup in spawned task |
| SecretString logged in error messages | Info Disclosure | `thiserror` error messages must not reference the key value; use `ChatError::CredentialError("not configured")` not `format!("{}", key)` |

---

## Environment Availability

| Dependency | Required By | Available | Notes |
|------------|------------|-----------|-------|
| Rust toolchain | All Cargo builds | Assumed present | Existing Phase 1 passed |
| `OPENROUTER_API_KEY` env var | Phase 2 runtime | Unknown | Must be set by developer; stubbed to None if absent |
| Internet access | OpenRouter SSE | Required at runtime | Not relevant for unit tests |
| Tauri v2 CLI | `cargo tauri dev` | Assumed present | Phase 1 used it |

**Missing dependencies with no fallback:**
- `OPENROUTER_API_KEY` — without it, `chat_send` returns `ChatError::CredentialError("OPENROUTER_API_KEY not configured")`. Implementer must set this in their shell before testing live streaming.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Capability permission key format is `"allow-chat-send"` / `"allow-chat-cancel"` | Capabilities JSON | Build will reject unknown permission key; easy to fix at build time |
| A2 | `reqwest` 0.13 uses rustls on Windows by default (no native-tls feature needed) | Cargo.toml | TLS handshake failure to openrouter.ai; fix: add `features = ["json", "stream", "rustls-tls"]` explicitly |
| A3 | `AppState` can be made `Clone` or wrapped in `Arc` for spawn sharing | Cancellation | Spawn pattern needs adjustment; alternative is `AppHandle` capture |
| A4 | `#[serde(tag = "type", rename_all = "PascalCase")]` produces `{ "type": "Delta", ... }` that the frontend TypeScript union correctly discriminates | Channel API | Frontend event handler never matches; rename_all value change required |
| A5 | `tokio = { features = ["sync"] }` is the correct feature name for `tokio_util::sync::CancellationToken` | Cargo.toml | Compile error; check tokio feature list |

---

## Open Questions

1. **AppState Arc pattern for spawned tasks**
   - What we know: `tauri::State<'_>` has a lifetime that prevents move into `tokio::spawn`
   - What's unclear: Whether to wrap `AppState` in `Arc` and register `Arc<AppState>`, or use `AppHandle` to re-acquire state
   - Recommendation: Use `app_handle: tauri::AppHandle` as an additional parameter in `chat_send`; call `app_handle.state::<AppState>()` inside the spawned task (AppHandle is `'static`-safe)

2. **`secrecy::SecretString` Clone for spawn**
   - What we know: `SecretString` doesn't impl `Clone` by default
   - What's unclear: Whether `secrecy 0.10.x` added `Clone` under the `alloc` feature
   - Recommendation: Use the expose-then-rewrap pattern from Section 4; don't assume Clone

3. **Exact capability permission key for custom commands**
   - What we know: Existing entries use `"allow-get-active-surface"`
   - What's unclear: Whether Tauri auto-generates these or they must match exactly
   - Recommendation: After first `cargo tauri dev` with new commands registered, check for capability errors in the output and use the exact key the error message mentions

---

## Sources

### Primary (HIGH confidence)
- `docs.rs/tauri/latest/tauri/ipc/struct.Channel.html` — Channel struct API, send() method
- `v2.tauri.app/develop/calling-frontend/` — Channel TypeScript usage pattern
- `docs.rs/reqwest/latest/reqwest/struct.Response.html` — bytes_stream(), chunk() methods
- `docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html` — CancellationToken API
- `docs.rs/secrecy/latest/secrecy/index.html` — SecretString, ExposeSecret
- `crates.io/api/v1/crates/{reqwest,secrecy,tokio-util,futures-util}` — version and legitimacy
- `openrouter.ai/docs/api/api-reference/chat/send-chat-completion-request` — request format
- `openrouter.ai/docs/api/reference/streaming` — SSE format, comment handling, [DONE]

### Secondary (MEDIUM confidence)
- WebSearch: CancellationToken HashMap pattern in Tauri AppState — confirmed against official tokio docs
- WebSearch: reqwest SSE bytes_stream with StreamExt — confirmed against official reqwest docs

### Tertiary (LOW confidence — see Assumptions Log)
- Capability permission key format `"allow-chat-send"` — inferred from existing entries in `main.json`

---

## Metadata

**Confidence breakdown:**
- Tauri Channel API: HIGH — verified against official v2 docs
- reqwest SSE: HIGH — verified against official docs; pattern is idiomatic futures-util
- CancellationToken: HIGH — verified against official tokio-util docs
- secrecy: HIGH — verified against docs.rs
- OpenRouter API shape: HIGH — verified against official OpenRouter docs
- Capability key format: LOW — inferred, must verify at build time

**Research date:** 2026-06-14
**Valid until:** 2026-07-14 (stable APIs; OpenRouter endpoint format unlikely to change)
