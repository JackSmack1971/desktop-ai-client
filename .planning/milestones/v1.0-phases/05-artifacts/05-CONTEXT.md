# Phase 5: Artifacts - Context

**Gathered:** 2026-06-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Provide sandboxed artifact previews that remain safe and usable under hostile content. This phase implements:

- `security::artifact_sandbox` — HTML sanitization (strip scripts, event handlers) plus CSP policy string generation. Belt-and-suspenders: backend sanitizes the content, frontend iframe enforces strict CSP as a second layer.
- `artifacts::detector` — backend-owned classifier called from `ipc::chat` after stream completion. Determines whether model output is an artifact (HTML/SVG/code block) or a plain message.
- `ArtifactReady` channel event — emitted through the existing `ChatEvent` channel when an artifact is detected. Frontend reacts to typed events only; it never classifies raw deltas.
- `ipc::artifacts` — minimal command surface: `artifact_get` (fetch sanitized artifact content for display or reload), `artifact_dismiss` (mark artifact dismissed). May expand; follow existing IPC patterns.
- SQLite `artifacts` table — persisted artifact revisions linked to `conversations`/`messages`. Backend is source of truth; frontend receives `srcdoc` content for display only.
- `ArtifactsSurface.svelte` — host-UI chrome with stop/reload controls (outside the iframe). Owns the artifact lifecycle controls; Chat surface shows only an artifact indicator + shortcut link.
- Sandboxed `<iframe>` inside `ArtifactsSurface.svelte` — static content rendered via `srcdoc` with strict `sandbox` attribute. Phase 5 hosts: HTML, SVG, plain text, and syntax-highlighted code.
- `chat_send` attachment token resolution — backend accepts `attachments: Option<Vec<TokenId>>` and resolves tokens to file content before calling the provider (completing Phase 4 D-06). No attachment picker UI in Phase 5.

This phase does NOT include: executable JavaScript artifacts (deferred; the iframe sandbox and CSP block all script execution), Tauri `WebviewWindow` for artifacts (deferred to Phase 6+ when executable artifacts are introduced), attachment picker UI in the frontend, persisted attachment metadata beyond the token resolution, or the Phase 6 command-inventory release verifier.

</domain>

<decisions>
## Implementation Decisions

### Sandbox Mechanism

- **D-01:** Phase 5 uses a sandboxed `<iframe>` with `srcdoc` inside `ArtifactsSurface.svelte` as the isolation boundary. The iframe has `sandbox="allow-same-origin"` (or more restrictive) and a strict CSP applied via a meta tag or by wrapping the srcdoc in a minimal HTML wrapper. No secondary Tauri `WebviewWindow` in Phase 5 — that upgrade path is deferred until executable JavaScript artifacts are introduced.
- **D-02:** `security::artifact_sandbox` is responsible for both (1) sanitizing the raw artifact HTML (stripping `<script>` tags, inline event handlers like `onclick`, `onerror`, etc., `javascript:` URIs) and (2) generating the `Content-Security-Policy` string applied to the sandboxed content. This is a belt-and-suspenders approach: backend sanitizes before content crosses IPC, and the iframe CSP enforces the constraint at the browser level.
- **D-03:** The iframe CSP for Phase 5 must block: `script-src 'none'`, `connect-src 'none'`, `form-action 'none'`, popups blocked. Allowed: inline styles (`style-src 'unsafe-inline'` for basic styling), `data:` and `blob:` URIs for images and fonts only. No external resources (no CDN images, no web fonts via URL). Production builds must fail closed — if CSP cannot be applied, content must not be rendered.

### Artifact Content Scope

- **D-04:** Phase 5 renders four content types inside the sandboxed iframe: (1) HTML (sanitized), (2) SVG, (3) plain text, (4) syntax-highlighted code. All are static — no JavaScript execution. Scripts, event handlers, network access, and all executable artifact behavior are blocked and deferred to a future phase.
- **D-05:** Artifact detection is backend-owned. `artifacts::detector` is called from `ipc::chat` after stream completion (after the `ChatEvent::Done` event is generated) and classifies the complete model output. If an artifact is detected, the backend stores it in the `artifacts` SQLite table and emits `ArtifactReady` through the existing chat channel. Frontend receives typed events only.

### Stop/Reload Controls

- **D-06:** The `ArtifactsSurface.svelte` chrome owns the real stop and reload controls. These controls are in the host Svelte UI, outside the sandboxed iframe — they cannot be blocked or overridden by the artifact content. The Chat surface shows only a minimal artifact indicator (e.g., "Artifact ready") with a link that navigates to the Artifacts surface. This keeps lifecycle controls outside the sandbox while preserving good UX.
- **D-07:** Reload always re-fetches the artifact revision from the backend via `artifact_get`, re-runs sanitization/wrapping in `security::artifact_sandbox`, and replaces the iframe `srcdoc`. No cached content is used by the frontend. No JavaScript restarts because Phase 5 artifacts do not execute JavaScript.
- **D-08:** Stop means dismiss/reset the static preview: the frontend sets the iframe `srcdoc` to empty (or navigates to `about:blank`), aborts any in-flight `artifact_get` IPC call to prevent stale content from appearing, and marks the artifact as dismissed in the artifacts store (`idle`/`dismissed` state). Stop does not navigate away from the Artifacts surface.

### Chat-to-Artifact Wiring

- **D-09:** `ArtifactReady` is a new variant in the existing `ChatEvent` tagged enum (alongside `Delta`, `Done`, `Error`). After stream completion and artifact detection, the backend emits `ChatEvent::ArtifactReady { artifact_id: Uuid, content_type: ArtifactContentType, preview: String }` through the chat channel. `preview` is the sanitized srcdoc ready for display. The frontend artifacts store subscribes to the chat channel and routes `ArtifactReady` to the Artifacts surface.
- **D-10:** Artifacts are persisted in a new `artifacts` SQLite table with `conversation_id` FK (and optionally `message_id`). The backend re-serves artifact content on `artifact_get` by re-running sanitization from the stored revision. Frontend receives the sanitized preview; it does not hold the unsanitized source.
- **D-11:** Phase 5 completes backend attachment token resolution in `chat_send`. The `chat_send` handler accepts `attachments: Option<Vec<TokenId>>` (Phase 4 already established the token mint/resolve/revoke surface). Backend resolves tokens to file content before constructing the provider request. No attachment picker UI is added in Phase 5 — that belongs in a future UX phase.

### Claude's Discretion

- Schema for the `artifacts` SQLite table (columns: `id`, `conversation_id`, `message_id`, `content_type`, `raw_source`, `created_at` — suggest this shape but Claude may refine based on query patterns)
- Exact `ArtifactContentType` enum variants (e.g., `Html`, `Svg`, `PlainText`, `Code { language: String }`)
- IPC error enum naming for artifact errors (follow `ShellError` from `ipc/app_shell.rs`)
- The exact `sandbox` attribute values on the iframe element (minimize allow-list; `allow-same-origin` may or may not be needed for srcdoc)
- Whether `artifact_dismiss` is a separate IPC command or a frontend-only state update (likely frontend-only since persistence of "dismissed" status is not required)

</decisions>

<canonical_refs>

## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope and Requirements

- `.planning/ROADMAP.md` — Phase 5 goal, success criteria (ARTF-01, ARTF-02, ARTF-03)
- `.planning/REQUIREMENTS.md` — ARTF-01, ARTF-02, ARTF-03 definitions

### Architecture and Patterns

- `.planning/codebase/ARCHITECTURE.md` — File intake data flow (§Data Flow: File Intake; artifact_sandbox is listed under security/), IPC boundary enforcement, command registration invariant (§Tauri Command Surface), error handling patterns (§Error Handling), threading/lock ordering (§Architectural Constraints), anti-patterns (§Anti-Patterns)
- `src-tauri/src/ipc/app_shell.rs` — Canonical IPC patterns: `ShellError` typed error enum with `thiserror + serde`, `assert_main_window` window-label enforcement. New `ArtifactsError` must follow the same serialization shape.
- `src-tauri/src/ipc/chat.rs` — Phase 2/3 output. `ChatEvent` enum lives here; `ArtifactReady` variant must be added to this enum. `chat_send` signature extended with `attachments: Option<Vec<TokenId>>`.

### Security and Privacy Authority

- `docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md` — **Primary authority** for IPC boundary invariants and isolation requirements. Contains rules on sandboxing, hostile renderer behavior, and content isolation. Read before designing any artifact isolation interface.
- `docs/privacy-boundaries.md` — Privacy boundary definitions; artifact content (which may contain sensitive user data) must stay backend-owned for sanitization.

### Phase 4 Context (upstream decisions Phase 5 must honor)

- `.planning/phases/04-privacy/04-CONTEXT.md` — D-04/D-05/D-06 (file token system; Phase 5 must wire `attachments: Option<Vec<TokenId>>` into `chat_send`), D-10 (`ipc::privacy` command surface; `privacy_*` patterns to follow), D-11 (SettingsSurface scope was narrow — Phase 5 does not expand Settings)
- `src-tauri/src/security/file_tokens.rs` — Phase 4 output. `mint_token`, `resolve_token`, `revoke_token` against `AppState.file_tokens`. Phase 5 `chat_send` calls `resolve_token` for each attachment.

### Phase 2 Context (streaming channel design)

- `.planning/phases/02-routing/02-CONTEXT.md` — D-01/D-02/D-03 (ChatEvent channel design). `ArtifactReady` must be added as a new `ChatEvent` variant following the same tagged enum shape.

### Scaffolded Files to Implement

- `src-tauri/src/security/artifact_sandbox.rs` — Scaffold; implement HTML sanitizer (strip scripts, event handlers) and CSP policy string builder.
- `src/lib/components/surfaces/ArtifactsSurface.svelte` — Scaffold; implement host chrome (stop/reload controls, artifact indicator) + sandboxed iframe with strict CSP.
- `src-tauri/src/ipc/chat.rs` — Extend `ChatEvent` enum with `ArtifactReady` variant; extend `chat_send` with `attachments: Option<Vec<TokenId>>`.

</canonical_refs>

<code_context>

## Existing Code Insights

### Reusable Assets

- `src-tauri/src/ipc/app_shell.rs` — `assert_main_window()` helper; `ShellError` enum with `thiserror + serde + #[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]` derive pattern. Copy for `ArtifactsError`.
- `src-tauri/src/ipc/chat.rs` — `ChatEvent` tagged enum and `chat_send` channel pattern (Phase 2 output). `ArtifactReady` added here as a new variant; the channel already exists.
- `src-tauri/src/security/file_tokens.rs` — Phase 4 output: `resolve_token()` callable from `chat_send` to resolve attachment tokens before provider request. Phase 5 calls this function.
- `src-tauri/src/storage/sqlite.rs` — `SqlitePool::with_conn()` pattern. New `ArtifactStore` (typed domain store) uses this pattern — never call `with_conn` from IPC handlers directly.
- `src-tauri/src/storage/migrations.rs` — Append-only migration runner. New migration for `artifacts` table must be added here in strictly ascending order.
- `src/lib/stores/surface.ts` — `normalizeIpcError()` and surface store patterns. Frontend artifacts store should use the same IPC error normalization pattern.

### Established Patterns

- **IPC error shape:** `{ code: "SCREAMING_SNAKE_CASE", message: string }` — all Phase 5 error enums must serialize to this shape.
- **Window-label enforcement:** Every artifacts command validates caller via `policy_check()` (Phase 4's `security::command_policy` replaces `assert_main_window`).
- **Typed domain stores:** `ConversationStore`/`MessageStore` (Phase 3) are the canonical model. New `ArtifactStore` wraps `SqlitePool` and exposes domain-specific methods.
- **Migration ordering:** Append-only, strictly ascending `id`. New migration for `artifacts` is the next entry after Phase 3's last migration.
- **Lock ordering:** Shell lock before SQLite lock. Artifact store operations that touch both must maintain this order.
- **Command registration invariant:** Every new command must appear in `tauri::generate_handler![...]` (main.rs) AND a `src-tauri/capabilities/*.json` grant (and eventually `security/command-inventory.toml` for Phase 6).

### Integration Points

- `src-tauri/src/main.rs` — Must register all Phase 5 IPC commands in `tauri::generate_handler![...]`.
- `src-tauri/src/app_state.rs` — May need extension if artifact store handle is managed separately from the SQLite pool (likely not — `ArtifactStore` wraps the existing pool).
- `src-tauri/capabilities/main.json` — Must add allow grants for all new Phase 5 commands.
- `src-tauri/src/ipc/chat.rs` — `ChatEvent` enum and `chat_send` handler both extended in Phase 5.
- `src/lib/components/surfaces/ArtifactsSurface.svelte` — Scaffold promoted to real component. Chat surface receives a new artifact indicator that navigates to Artifacts surface.

</code_context>

<specifics>
## Specific Ideas

- The iframe sandbox should be as restrictive as possible for Phase 5. The user explicitly chose `script-src 'none'`, `connect-src 'none'`, `form-action 'none'` — no exceptions. Only `data:` and `blob:` URIs for images/fonts. If there is any doubt about whether a CSP directive can be relaxed, it must not be.
- `security::artifact_sandbox` is a belt-and-suspenders design: backend sanitization runs first (strips `<script>`, inline event handlers, `javascript:` URIs), then the sanitized content is wrapped with the CSP meta tag before being handed to the frontend. Even if one layer has a bug, the other provides defense.
- The user confirmed: "Production builds must fail closed — if CSP cannot be applied, content must not be rendered." Implement this as a guard in `ArtifactsSurface.svelte`: if the sanitized srcdoc is absent or an error occurs during fetch, show an error state, not a fallback render.
- Stop/reload controls are in the Artifacts surface chrome (host UI), not inside the iframe. They cannot be blocked by artifact content. This is an explicit safety requirement — do not put lifecycle controls inside the sandboxed content.
- `ArtifactReady` is emitted after stream completion (after `ChatEvent::Done`), not during streaming. Artifact detection runs on the complete model output, not on streaming deltas. This means the user sees the artifact appear after the stream ends, not while it's streaming — acceptable for Phase 5.
- Phase 5 artifact store's `artifact_get` always re-runs `security::artifact_sandbox::sanitize()` on the stored raw source before returning the srcdoc. This ensures the CSP policy and sanitization are always current, even if sanitization logic is improved in a future patch.

</specifics>

<deferred>
## Deferred Ideas

- **Executable JavaScript artifacts** — Sandboxed JS execution inside a Tauri `WebviewWindow` with no IPC grants. Requires a separate window/webview with its own capability profile. Deferred to Phase 6+ with an explicit user-controlled toggle.
- **Tauri WebviewWindow for artifacts** — Inline WebviewWindow panel embedded in the main window for true process-level isolation. The iframe-based approach in Phase 5 is an intentional stepping stone. Upgrade when executable artifacts are introduced.
- **Attachment picker UI** — The frontend file selection flow (open picker → get token → attach to message). Phase 5 completes only the backend token resolution in `chat_send`; the UX for selecting and attaching files is deferred.
- **Persisted attachment metadata** — Safe metadata (filename, type, size) stored in SQLite for conversation history display. Deferred from Phase 4 and still deferred here; token authority and source paths must never be persisted.
- **Artifact revision history** — Multiple revisions per artifact (user edits, regenerations). Phase 5 stores one revision per artifact; multi-revision tracking deferred.
- **Artifact export / download** — User saves an artifact to disk. Requires file write authority; deferred to a future phase.
- **Streaming artifact preview** — Show artifact content while it's still streaming (for long HTML generation). Deferred; Phase 5 detects and displays after stream completion only.

None — discussion stayed within phase scope.

</deferred>

---

_Phase: 05-artifacts_
_Context gathered: 2026-06-15_
