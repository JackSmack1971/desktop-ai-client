# Phase 5: Artifacts - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-15
**Phase:** 05-artifacts
**Areas discussed:** Sandbox container, Artifact content scope, Stop/reload controls, Chat-to-artifact wiring

---

## Sandbox Container

### How should artifact content be isolated from the host UI?

| Option | Description | Selected |
|--------|-------------|----------|
| Tauri WebviewWindow | Secondary Tauri window/webview with its own capability profile — no IPC commands granted, CSP enforced at Tauri config level. Backend manages lifecycle via security::artifact_sandbox. | |
| Sandboxed `<iframe>` in main WebView | An `<iframe sandbox>` element inside ArtifactsSurface.svelte. Browser sandbox blocks same-origin access, localStorage, and top-level navigation. | |
| You decide | Let Claude choose based on architecture constraints and threat model. | |

**User's choice:** "Use option 1 for executable artifacts. A sandboxed iframe remains acceptable only for static/constrained preview, not as the primary isolation boundary for generated JavaScript."
**Notes:** Nuanced response — implied a tiered approach: iframe for static Phase 5, WebviewWindow for executable artifacts later. Triggered the next question to clarify Phase 5 scope.

---

### Does Phase 5 need to support executable artifacts (generated JavaScript that runs)?

| Option | Description | Selected |
|--------|-------------|----------|
| Static only — no JS execution | Artifacts are rendered HTML/SVG/markdown with scripts stripped or blocked by CSP. `<iframe sandbox>` without `allow-scripts` is sufficient. | |
| Executable JS in Phase 5 | AI can generate runnable artifacts (React components, chart code). Needs a Tauri WebviewWindow with allow-scripts but no IPC. | |
| Static now, executable later | Phase 5 implements static preview with WebviewWindow infrastructure in place. Phase 6+ enables script execution behind a toggle. | ✓ |

**User's choice:** Option 3 — static-only Phase 5, infrastructure for WebviewWindow deferred.
**Notes:** Keeps Phase 5 tractable. Executable JS deferred to Phase 6+ behind an explicit user-controlled toggle with no-IPC isolation.

---

### How is the sandboxed preview hosted inside the Tauri app?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline WebviewWindow panel | Secondary WebviewWindow embedded as a panel inside the main window. Rust manages lifecycle. Host window never freezes. | |
| Detached popup window | Artifact opens in a separate OS window. Simple Rust lifecycle, but disruptive UX. | |
| Sandboxed `<iframe>` with strict CSP | Static-only: `<iframe sandbox>` without allow-scripts inside ArtifactsSurface.svelte. security::artifact_sandbox.rs enforces CSP. Simpler for Phase 5, upgrade path later. | ✓ |

**User's choice:** Option 3 — iframe inside ArtifactsSurface.svelte for Phase 5.
**Notes:** WebviewWindow deferred explicitly until executable artifacts are introduced.

---

### What's the scope of security::artifact_sandbox.rs in Phase 5?

| Option | Description | Selected |
|--------|-------------|----------|
| CSP policy builder | Computes the Content-Security-Policy header string for the artifact iframe. | |
| Content validator / sanitizer | Validates and sanitizes raw HTML artifact content before it crosses to the frontend. | |
| Both — sanitize on backend + CSP enforced on frontend | Belt and suspenders: artifact_sandbox.rs sanitizes the HTML payload, and the frontend iframe enforces CSP as a second layer. | ✓ |

**User's choice:** Option 3 — defense in depth.
**Notes:** "Phase 5 artifact_sandbox sanitizes/wraps static artifact content on the backend and the frontend iframe enforces a strict no-script CSP."

---

## Artifact Content Scope

### What artifact content types does Phase 5 need to render?

| Option | Description | Selected |
|--------|-------------|----------|
| HTML + SVG | Render raw HTML (sanitized) and SVG in the iframe. No markdown conversion layer needed. | |
| Markdown only | Frontend converts markdown to HTML client-side before loading into the iframe. | |
| HTML + SVG + plain text/code | Everything above plus plain text and syntax-highlighted code blocks rendered as HTML. | ✓ |

**User's choice:** Option 3 — HTML, SVG, plain text, and code; all static.
**Notes:** "Scripts, event handlers, network access, and executable artifact behavior stay blocked/deferred."

---

### What must the iframe CSP explicitly block?

| Option | Description | Selected |
|--------|-------------|----------|
| Strict block | `script-src 'none'`, `connect-src 'none'`, `form-action 'none'`, allow-popups blocked. Allow: inline styles, data/blob images and fonts. No exceptions. | ✓ |
| Block scripts + network, allow some external resources | No scripts, no network XHR/fetch, but allow external images and fonts. | |
| You decide | Let Claude design the CSP based on the static-only constraint. | |

**User's choice:** Option 1 — maximum restriction.
**Notes:** "No scripts, no network, no forms, no popups, no external resources. Allow only inline styles plus data/blob images and data fonts."

---

### Who detects that assistant output contains an artifact vs. a plain message?

| Option | Description | Selected |
|--------|-------------|----------|
| Backend — in ipc::chat | The chat_send handler or a new ipc::artifacts command inspects model output and classifies it before sending to the frontend. Frontend receives typed events. | ✓ |
| Frontend — chat store heuristics | The frontend chat store inspects streaming deltas for artifact signals and routes them to the artifacts surface. | |
| You decide | Let Claude design the artifact detection and routing based on IPC boundary rules. | |

**User's choice:** Option 1 — backend-owned detection.
**Notes:** "Artifact detection is backend-owned, preferably in artifacts::detector called from ipc::chat after stream completion or after a complete artifact block is available."

---

## Stop/Reload Controls

### Where do the stop and reload controls live?

| Option | Description | Selected |
|--------|-------------|----------|
| In the Artifacts surface shell (host UI) | Stop/reload buttons in ArtifactsSurface.svelte's chrome, outside the iframe. Controls in Svelte renderer, not inside sandboxed content. | |
| In the Chat surface header | Controls appear in Chat surface when an artifact is active — same toolbar as the cancel button for streaming. | |
| Both — Artifacts surface has controls, Chat surface has a shortcut | Primary controls in ArtifactsSurface toolbar; Chat shows a minimal indicator with a click-to-open link. | ✓ |

**User's choice:** Option 3 — Artifacts surface owns controls, Chat shows indicator.
**Notes:** "Keeps lifecycle controls outside the sandbox while preserving good UX."

---

### How does 'reload' work for a static artifact (no live execution to restart)?

| Option | Description | Selected |
|--------|-------------|----------|
| Re-fetch from backend and re-inject | Reload invokes `artifact_get` IPC command to re-fetch sanitized artifact content, then replaces iframe src blob/dataURL. | ✓ |
| Frontend re-injects the last received content | Artifacts store re-stamps the iframe src from cached content without an IPC round-trip. | |
| You decide | Let Claude design reload behavior based on IPC boundary rules. | |

**User's choice:** Option 1 — always re-fetch from backend.
**Notes:** "Reload always re-fetches the artifact revision from the backend, re-runs static artifact sanitization/wrapping, and replaces the iframe srcdoc."

---

### What does 'stop' mean for a static artifact in Phase 5?

| Option | Description | Selected |
|--------|-------------|----------|
| Clear the iframe / reset to empty state | Stop sets iframe src to `about:blank` and marks artifact as dismissed in the artifacts store. | ✓ |
| Close the Artifacts surface and navigate away | Stop navigates the surface store back to Chat. | |
| Abort an in-progress artifact IPC fetch | Stop cancels the `artifact_get` IPC call if one is in flight. | |

**User's choice:** Combined behavior — clear iframe + abort in-flight fetch.
**Notes:** "Stop means dismiss/reset the static preview: clear the iframe and mark the artifact preview idle/dismissed. If a reload/fetch is in progress, invalidate or abort it so stale content cannot reappear."

---

## Chat-to-Artifact Wiring

### How does an artifact generated by the AI get routed to the Artifacts surface?

| Option | Description | Selected |
|--------|-------------|----------|
| New ArtifactReady channel event from ipc::chat | After stream completion, artifacts::detector classifies the output and emits an ArtifactReady event through the existing ChatEvent channel. | ✓ |
| New ipc::artifacts IPC command (separate from chat) | A dedicated `artifacts_create` command called by the backend after detection, or by the frontend after receiving Done. | |
| You decide | Let Claude design the wiring based on existing ChatEvent channel patterns. | |

**User's choice:** Option 1 — ArtifactReady through existing chat channel.
**Notes:** "Backend chat detects and creates artifacts after stream completion, then emits ArtifactReady through the existing chat channel. The frontend only reacts to typed events."

---

### Are artifact revisions persisted in SQLite, or ephemeral?

| Option | Description | Selected |
|--------|-------------|----------|
| Persisted — linked to the conversation row | Artifacts stored in an `artifacts` SQLite table with `conversation_id` FK. Backend can re-serve them on reload. | ✓ |
| Ephemeral — in-memory only, lost on quit | Artifact content held in AppState memory. Simple, no migration needed. | |
| You decide | Let Claude design based on Phase 3 history storage pattern. | |

**User's choice:** Option 1 — persisted in SQLite.
**Notes:** "Phase 5 persists artifacts and artifact revisions in SQLite, linked to conversations/messages. The frontend receives preview srcdoc, but the backend remains the source of truth."

---

### Does Phase 5 complete the attachment token wiring into chat_send deferred from Phase 4?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — include attachment tokens in Phase 5 | Phase 5 extends `chat_send` to accept `attachments: Option<Vec<TokenId>>` and resolves them before calling the provider. | |
| No — keep attachment wiring deferred | Phase 5 focuses on artifact preview only. Attachment tokens remain for a future phase. | |
| Partial — wire token resolution only, no attachment UI | Backend accepts attachment tokens in `chat_send` (completing Phase 4 D-06), but no attachment picker UI in Phase 5. | ✓ |

**User's choice:** Option 3 — backend token resolution only.
**Notes:** "Phase 5 completes backend token resolution in chat_send, but does not expose the full attachment picker/send UX yet."

---

## Claude's Discretion

- Schema for the `artifacts` SQLite table (columns suggested: `id`, `conversation_id`, `message_id`, `content_type`, `raw_source`, `created_at`)
- Exact `ArtifactContentType` enum variants (e.g., `Html`, `Svg`, `PlainText`, `Code { language: String }`)
- IPC error enum naming for artifact errors (follow `ShellError` shape)
- Exact `sandbox` attribute values on the iframe element (minimize allow-list)
- Whether `artifact_dismiss` is a separate IPC command or a frontend-only state update

## Deferred Ideas

- Executable JavaScript artifacts (Phase 6+ with user-controlled toggle)
- Tauri WebviewWindow for artifacts (when executable artifacts are introduced)
- Attachment picker UI (future UX phase)
- Persisted attachment metadata (filename, type, size in SQLite)
- Artifact revision history (multiple revisions per artifact)
- Artifact export / download to disk
- Streaming artifact preview (display while streaming, not just after completion)
