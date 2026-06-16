# Phase 2: Routing - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 02-routing
**Areas discussed:** Streaming delivery, Cancellation design, Credential provisioning, Chat request shape

---

## Streaming Delivery

**Q1: How should streaming chunks get from Rust to Svelte?**

| Option | Description | Selected |
|--------|-------------|----------|
| Tauri events (emit/listen) | app.emit per token; listen in frontend. Most common Tauri pattern. | |
| Tauri v2 channels | Typed Channel<T> sender passed into IPC handler. More structured. | ✓ |
| Polling | Frontend polls via IPC. Simple but choppy, not recommended. | |

**User's choice:** Tauri v2 channels

---

**Q2: What payload should each channel message carry?**

| Option | Description | Selected |
|--------|-------------|----------|
| Text delta only | `{ delta: string }` with sentinel done message. Minimal overhead. | |
| Full state per chunk | Full accumulated text + delta + metadata per message. O(n²) data. | |
| Structured event type | Tagged enum: Delta \| Done \| Error. Most explicit contract. | ✓ |

**User's choice:** Structured event type — `ChatEvent::Delta { text } \| ChatEvent::Done { usage } \| ChatEvent::Error { code, message }`

---

**Q3: Should chat_send also return a result, or channel-only signaling?**

| Option | Description | Selected |
|--------|-------------|----------|
| Channel-only signaling | chat_send returns () immediately. All terminal state via channel. | ✓ |
| Dual signal | chat_send returns final status AND channel carries deltas. Two paths to reconcile. | |

**User's choice:** Channel-only signaling

---

## Cancellation Design

**Q1: How should the user cancel an in-flight stream?**

| Option | Description | Selected |
|--------|-------------|----------|
| Separate chat_cancel IPC command | Cancel button calls invoke('chat_cancel', { request_id }). Explicit, testable. | ✓ |
| Auto-cancel on new message | Sending new prompt cancels previous stream. Implicit. | |
| Frontend closes channel | Svelte drops listener; Rust detects closed channel. Indirect abort signal. | |

**User's choice:** Separate chat_cancel IPC command with request_id

---

**Q2: What does the frontend show while a request is in flight?**

| Option | Description | Selected |
|--------|-------------|----------|
| Streaming bubble + cancel button | Assistant bubble appears immediately, fills token by token. Cancel in input area. | |
| Spinner until first token | Spinner during wait, then streaming bubble. Avoids blank bubble before TTFT. | |
| You decide | Leave to Claude's discretion. | |

**User's choice (freeform):** Hybrid approach — immediate placeholder with skeleton/thinking state on submit; transition to streaming text bubble on first token received; persistent cancel button anchored in input area (decoupled from bubble), visible throughout request.

**Notes:** User emphasized the cancel button should be decoupled from the streaming bubble itself for input area stability.

---

**Q3: What happens to partial response on cancel?**

| Option | Description | Selected |
|--------|-------------|----------|
| Keep partial + mark cancelled | Partial text preserved with visual "(Cancelled)" badge. | ✓ |
| Discard partial response | Bubble disappears on cancel. | |
| You decide | Leave to Claude's discretion. | |

**User's choice (freeform):** Keep partial response. Freeze streaming component immediately. Append amber-colored badge ("(Cancelled)" or "[Halted]") at end of text. Dim text slightly to distinguish from completed responses ("command center aesthetic"). Persist with `status: "incomplete"` so subsequent model inferences understand truncation was user-initiated, not model-initiated.

---

## Credential Provisioning

**Q1: How should the API key be available in Phase 2?**

| Option | Description | Selected |
|--------|-------------|----------|
| Environment variable at startup | Read OPENROUTER_API_KEY at startup, hold in AppState. Simple. | |
| Thin secrets stub now | Implement security::secrets with get_provider_key(); Phase 4 swaps internals. | ✓ |
| Mock/stub provider | No real API key needed; stub returns fake tokens. | |

**User's choice:** Thin secrets stub in Phase 2. Rationale: the hardened spec's invariant is not just "don't show the key in UI" — backend owns secret retrieval, and stream commands must NEVER accept an api_key parameter. Aligns with adversarial spec requirement that the backend retrieves credentials internally.

---

**Q2: What interface should the secrets stub expose?**

| Option | Description | Selected |
|--------|-------------|----------|
| get_provider_key(provider: ProviderId) -> Result<String, SecretsError> | Typed provider enum, String return. | |
| get_key(key_name: &str) -> Result<String, SecretsError> | Generic string key, loses type safety. | |

**User's choice (refined):** Use typed ProviderId enum but return `SecretString` (not `String`) to zeroize on drop. Also expose `get_credential_status(provider: ProviderId) -> CredentialStatus` for UI feedback without revealing the key value.

---

## Chat Request Shape

**Q1: What does chat_send accept?**

| Option | Description | Selected |
|--------|-------------|----------|
| Messages array + optional model override | `{ messages, model?, conversation_id? }`. Full history from frontend. | ✓ |
| Latest user message only | `{ content, conversation_id? }`. Backend assembles history from SQLite (couples Phase 3). | |
| You decide | Leave to Claude's discretion. | |

**User's choice:** Messages array (required), plus optional: model, conversation_id, attachments, max_completion_tokens, temperature. Backend resolves defaults, creates internal IDs, retrieves provider key internally.

---

**Q2: How is the model selected when not overridden?**

| Option | Description | Selected |
|--------|-------------|----------|
| Backend config / hardcoded default | Static default in Rust config. Simple for Phase 2. | ✓ |
| Capability detection selects model | providers::capabilities queries OpenRouter API. Adds startup network call. | |
| User picks model in settings UI | Requires settings work alongside routing. | |

**User's choice:** Backend-configured default for Phase 2. Return the resolved model name in ChatEvent::Done. Dynamic capability-based selection deferred to a future phase.

---

**Q3: Does chat_send need a system_prompt field?**

| Option | Description | Selected |
|--------|-------------|----------|
| Backend-owned system prompt, not in IPC | Configured in AppConfig, prepended internally. | ✓ |
| Optional system_prompt field in chat_send | Frontend can customize per request. Potential security surface. | |
| No system prompt in Phase 2 | Skip entirely for now. | |

**User's choice:** Backend-owned system prompt. chat_send does not accept system_prompt. Future user-editable custom instructions must go through a settings-owned command with validation and UI disclosure — not through chat_send.

---

## Claude's Discretion

- Exact Rust crate for `SecretString` (likely `secrecy::Secret<String>`)
- Exact HTTP client for OpenRouter SSE (likely `reqwest` with streaming)
- `assert_main_window` equivalent for chat commands (follow app_shell pattern)
- `ChatError` enum variant naming (follow `ShellError` pattern)
- Exact mechanism for passing `request_id` back to frontend before streaming starts (Ack variant vs. other)

## Deferred Ideas

- Settings UI model picker
- Capability-based model selection via `providers::capabilities`
- Full secrets store with Stronghold/OS keychain (Phase 4)
- Conversation persistence — `conversation_id` wired but history write is Phase 3
