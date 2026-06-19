# Phase 2: Routing - Context

**Gathered:** 2026-06-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Route user prompts from the Svelte renderer through the Tauri IPC boundary to OpenRouter via SSE streaming transport. Deliver ordered partial output to the frontend via Tauri v2 channels, with cancellation support and typed error handling. Provider credentials remain backend-owned and invisible to the renderer throughout.

This phase does NOT include: full secrets store (Phase 4), conversation persistence (Phase 3), file attachments (Phase 4), settings UI model picker, or capability-based model selection.

</domain>

<decisions>
## Implementation Decisions

### Streaming Delivery

- **D-01:** Use Tauri v2 `Channel<ChatEvent>` (not Tauri events, not polling) as the streaming transport from Rust to Svelte.
- **D-02:** `ChatEvent` is a structured tagged enum with three variants:
  - `ChatEvent::Delta { text: String }` — incremental token text
  - `ChatEvent::Done { usage: Option<TokenUsage>, model: String }` — stream complete; includes resolved model name
  - `ChatEvent::Error { code: String, message: String }` — error or cancellation signal
- **D-03:** `chat_send` signals all terminal state (done, error, cancellation) exclusively through the channel. The frontend drops the channel listener after receiving `Done` or `Error`. No dual data paths.

### Cancellation Design

- **D-04:** Cancellation uses a separate `chat_cancel` IPC command. A `request_id` (backend-generated or frontend-generated UUID) identifies the in-flight request. The cancel button calls `invoke('chat_cancel', { request_id })`. Rust aborts the SSE connection and emits `ChatEvent::Error { code: "CANCELLED", message: "..." }` through the channel to close the listener cleanly.
- **D-05:** Loading UX is hybrid: on submit, insert an immediate placeholder (skeleton/thinking state). On first `ChatEvent::Delta`, transition to a streaming text bubble. A persistent cancel button remains anchored in the input area (not inside the message bubble) throughout the request.
- **D-06:** On cancellation: freeze the streaming component immediately, append an amber-colored visual badge ("(Cancelled)" or "[Halted]") at the end of the partial text, dim the text slightly to distinguish from completed responses. Persist the partial message with `status: "incomplete"` so subsequent model inferences understand the text was truncated by the user (not the model).

### Credential Provisioning

- **D-07:** Implement a thin `security::secrets` stub in Phase 2. Phase 4 replaces the internals (env-var → Stronghold/OS keychain) without changing callers.
- **D-08:** The stub exposes exactly two methods:
  - `get_provider_key(provider: ProviderId) -> Result<SecretString, SecretsError>` — typed `ProviderId` enum; `SecretString` zeroizes on drop
  - `get_credential_status(provider: ProviderId) -> CredentialStatus` — lets the UI show credential state without ever revealing the key
- **D-09:** Phase 2 backing store: read `OPENROUTER_API_KEY` from the process environment at startup; hold it in `AppState` behind a `Mutex`.
- **D-10 (hard invariant):** IPC commands MUST NEVER accept an `api_key` parameter. The backend retrieves credentials internally via `secrets.get_provider_key(provider)`. This is a non-negotiable constraint from the adversarial hardening spec. Any IPC handler that accepts a key value is a protocol violation.

### Chat Request Shape

- **D-11:** `chat_send` signature:
  ```
  Required: messages: Vec<ChatMessage>    // { role: "user"|"assistant", content: String }
  Optional: model: Option<String>         // defaults to backend-configured model if None
            conversation_id: Option<Uuid> // for Phase 3 history linking
            max_completion_tokens: Option<u32>
            temperature: Option<f32>
  ```
- **D-12:** System prompt is backend-owned and invisible to the IPC surface. It is configured in `AppConfig` (Rust), prepended internally during provider request construction, and never accepted from `chat_send`. Future user-editable custom instructions must go through a settings-owned command with validation, not through `chat_send`.
- **D-13:** Model selection defaults to a backend-configured constant (e.g., `anthropic/claude-sonnet-4-6`). The resolved model name is returned in `ChatEvent::Done { model }`. Dynamic capability-based model selection is deferred.
- **D-14:** The backend generates an internal `request_id` per `chat_send` call and uses it to register a cancellable task handle. This ID is returned either as part of an initial `ChatEvent::Ack { request_id }` variant or via another mechanism — the planner should determine the exact channel wiring for passing the ID back to the frontend before streaming begins.

### Claude's Discretion

- Exact Rust crate for `SecretString` (likely `secrecy::Secret<String>` from the `secrecy` crate)
- Exact HTTP client for OpenRouter SSE (likely `reqwest` with streaming; check `Cargo.toml` for existing deps)
- Window-label enforcement pattern for `chat_send` (follow the `assert_main_window` pattern from `ipc::app_shell`)
- Error enum naming for `ChatError` variants (follow `ShellError` from `ipc::app_shell.rs` as the established pattern)

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope and Requirements

- `.planning/ROADMAP.md` — Phase 2 goal, success criteria (ROUTE-01, ROUTE-02)
- `.planning/REQUIREMENTS.md` — ROUTE-01 and ROUTE-02 definitions (the two requirements this phase satisfies)

### Architecture and Patterns

- `.planning/codebase/ARCHITECTURE.md` — Chat Message data flow (§Data Flow), anti-patterns (§Anti-Patterns), error handling patterns (§Error Handling), IPC command registration invariant (§Tauri Command Surface)
- `.planning/codebase/INTEGRATIONS.md` — OpenRouter integration plan, Tauri security config, registered vs scaffolded IPC commands
- `src-tauri/src/ipc/app_shell.rs` — Canonical established patterns: `ShellError` typed error enum, `assert_main_window` window-label enforcement, optimistic-update rollback. **New `ChatError` and `chat_*` commands must follow these patterns.**

### Provider and Security Constraints

- `docs/provider-routing.md` — Provider routing focus areas: capability detection, routing policy, fallback behavior, provider drift handling
- `docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md` — **Adversarial hardening spec.** User explicitly invoked this during discussion. Contains the hard invariant: backend-only secrets, stream commands must never accept api_key, `stream_chat` retrieves credentials internally. Read before designing any IPC interface for this phase.
- `docs/privacy-boundaries.md` — Privacy boundary definitions relevant to credential handling
- `docs/threat-model.md` — Threat model for provider routing and hostile renderer behavior

### Codebase Maps

- `.planning/codebase/ARCHITECTURE.md` — (already listed above; emphasize §Layers for module dependency rules)
- `.planning/codebase/CONCERNS.md` — Phase 02 readiness gate checklist and 18 unimplemented scaffold modules

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `src-tauri/src/ipc/app_shell.rs` — `assert_main_window()` helper; `ShellError` enum with `thiserror` + `serde` derive pattern. Copy this pattern for `ChatError`.
- `src-tauri/src/storage/sqlite.rs` — `SqlitePool` with `with_conn()` pattern. `chat_send` will need to call into storage for conversation persistence (Phase 3 wires this — Phase 2 can stub or skip the storage write).
- `src/lib/stores/surface.ts` — `normalizeIpcError()` and optimistic-update rollback. Frontend chat store should follow the same error normalization pattern.

### Established Patterns

- **IPC error shape:** `{ code: "SCREAMING_SNAKE_CASE", message: string }` — enforced at IPC layer, normalized on frontend. `ChatError` must serialize to the same shape.
- **Window-label enforcement:** Every shell command validates caller via `assert_main_window`. Apply to all `chat_*` commands.
- **Dependency direction:** `ipc/` depends on `{providers, security, storage, telemetry}`. Backend modules must not import from `ipc/`. `chat_send` calls `providers::routing`, which calls `providers::openrouter`, which uses `providers::sse` and `security::secrets`.
- **Lock ordering:** shell lock acquired before sqlite lock. `chat_send` must not invert this.

### Integration Points

- `src-tauri/src/providers/` — All four modules are scaffold placeholders: `routing.rs`, `openrouter.rs`, `sse.rs`, `capabilities.rs`. This phase implements `routing`, `openrouter`, and `sse`; `capabilities` deferred.
- `src-tauri/src/security/secrets.rs` — Scaffold placeholder. This phase implements the thin stub.
- `src-tauri/src/ipc/chat.rs` — Scaffold placeholder. This phase implements `chat_send` and `chat_cancel`.
- `src-tauri/src/main.rs` — `tauri::generate_handler![]` must be updated to register new chat commands. Capabilities JSON (`src-tauri/capabilities/main.json`) must be updated with allow grants.

</code_context>

<specifics>
## Specific Ideas

- The user referenced the "adversarial hardening spec" (`docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md`) specifically when defining the credential invariant. Downstream agents should treat this doc as a primary authority for any IPC design decision in this phase.
- The user described the cancelled-message visual as: amber-colored badge ("(Cancelled)" or "[Halted]"), text dimmed relative to completed responses, decoupled from the streaming bubble itself — resembling a "command center aesthetic."
- The user described the `CredentialStatus` response from `get_credential_status()` as usable for frontend status feedback (e.g., showing whether a provider is configured) without revealing the key value.
- The user expects `ChatEvent::Done` to carry the resolved model name so the frontend can display which model answered.

</specifics>

<deferred>
## Deferred Ideas

- **Settings UI model picker** — user selects active model in the settings surface. Belongs in a future UI phase.
- **Capability-based model selection** — `providers::capabilities` inspects the OpenRouter model catalog and selects the best available model. Deferred; Phase 2 uses a static backend default.
- **Full secrets store** — Stronghold/OS keychain integration. Phase 4 replaces the env-var backing in the `security::secrets` stub.
- **Conversation persistence** — `conversation_id` field in `chat_send` is wired but history write is Phase 3.

None — discussion stayed within phase scope.

</deferred>

---

_Phase: 02-routing_
_Context gathered: 2026-06-14_
