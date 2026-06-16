# Optimal Technical Architecture for a Local-History Desktop AI Application

**Document status:** Adversarially hardened implementation specification, revision 5  
**Original source copied from:** `Tauri Svelte AI App Architecture.txt`  
**Adversarial edit date:** 2026-06-13
**Revision:** 5 — deny-by-inventory Tauri command exposure, Channel-first streaming, explicit release capability selection, request-time provider privacy constraints, no frontend Stronghold read surface, opaque file-token intake, programmatic `srcdoc` assignment, and expanded adversarial fixture coverage  
**Purpose:** This document preserves the original architectural direction—Tauri v2, Svelte 5, SQLite/FTS5, CodeMirror 6, and OpenRouter-compatible model routing—but removes unsafe claims, hardens the security model, and converts volatile assumptions into implementation gates that downstream agents must verify before coding. Revision 5 also removes any prescription that depends on unverified or underspecified framework behavior; where Tauri or another dependency provides a native mechanism, the selected version must prove it, and where it does not, this specification requires a project-owned verifier, fixture, or CI gate instead.

---

## Change Log of Surgical Edits

The original document was directionally strong but too implementation-dangerous in several places. The following corrections are now normative:

1. **Renamed the architecture from “local-first” to “local-history / cloud-inference by default.”** The app stores user data locally, but OpenRouter inference sends prompts and attachments to a remote provider unless a local backend is selected.
2. **Removed hardcoded OpenRouter free-tier model and quota assumptions.** Model inventory, modalities, context windows, pricing, and free limits must be discovered dynamically through OpenRouter metadata and revalidated at runtime.
3. **Prohibited frontend API-key handling.** The frontend must never pass the API key into a Tauri command or read it back from storage. The Rust backend retrieves credentials internally.
4. **Demoted `@tauri-apps/plugin-sql` from universal frontend database access to a constrained option.** Preferred persistence access is backend repository commands with typed schemas.
5. **Converted IPC security from general advice into a per-window capability contract.** Every sensitive command must be explicitly scoped to a trusted window or webview.
6. **Replaced naive streaming pseudocode with a real SSE parsing contract.** The backend must parse complete SSE events, handle partial UTF-8, stream IDs, `[DONE]`, cancellation, and errors.
7. **Replaced the naive FTS5 schema with an external-content FTS5 schema and required triggers/rebuild operations.**
8. **Hardened the Artifacts sandbox.** `srcdoc` iframe preview remains acceptable only with no Tauri IPC, no `allow-same-origin`, strict `postMessage` validation, CSP, output escaping, and kill/reload controls.
9. **Changed “60 fps” from a guarantee into a benchmark target.** It must be proven by test fixtures across long transcripts, Markdown, code blocks, KaTeX, and low-end hardware.
10. **Added release gates.** The architecture cannot be considered implemented until security, performance, quota, data, and bundle-size gates pass in CI.

11. **Added an explicit threat model and security invariants.** The document now names protected assets, adversary classes, trust boundaries, and non-negotiable failure conditions.
12. **Added updater, signing, and release-channel hardening.** A desktop app that can update itself is a supply-chain system; updater signing, channel separation, rollback policy, and artifact verification are now release blockers.
13. **Added attachment and file-ingestion hardening.** File uploads, PDFs, images, archives, and generated artifacts must pass size, type, path, decompression, metadata, and provider-disclosure gates before leaving the device.
14. **Added log, telemetry, crash-report, and prompt-retention rules.** Secrets, prompts, attachments, and generated artifacts must not leak through logs, crash dumps, traces, analytics, or screenshots.
15. **Added schema migration rollback and backup requirements.** SQLite migrations must be transactional, reversible where possible, tested against dirty WAL states, and covered by backup/export/restore fixtures.
16. **Added model/provider drift controls.** The UI must treat provider metadata as cached claims, not static truth, and must degrade safely when model capabilities, pricing, routing, moderation, or provider terms change.
17. **Added explicit production build hardening.** Development tools, permissive CSP, broad dev capabilities, local debug endpoints, and source maps containing sensitive paths are blocked from release builds.
18. **Added “definition of done” gates for downstream agents.** Implementation is not accepted merely because code compiles; it must pass privacy, security, migration, packaging, and adversarial fixture gates.
19. **Added a reviewed command-inventory requirement.** Registered commands must be explicitly listed in a source-controlled inventory and verified against Rust registration plus release capabilities; capability files remain necessary but are not sufficient. Production builds must keep `app.withGlobalTauri` disabled.
20. **Restricted Stronghold to Rust-owned or frontend-inaccessible secret storage.** Stronghold may not expose guest/frontend read permissions such as store-record reads to ordinary UI windows.
21. **Upgraded the streaming contract to full SSE grammar handling.** The backend must parse comments, `event`, `data`, `id`, and `retry` fields, concatenate multi-line `data:` fields, dispatch only on blank-line event boundaries, and then normalize provider deltas.
22. **Added Rust-only updater authentication and channel handling.** Private update-feed headers, bearer tokens, channel-routing decisions, and rollback authorization must be owned by Rust, never supplied by JavaScript.
23. **Added production provider-debug and retention controls.** Provider debug modes, provider metadata expansion, and content-bearing generation retrieval surfaces are default-off and banned in production unless explicitly approved by privacy review.
24. **Added custom endpoint SSRF and redirect controls.** User-configurable OpenAI-compatible endpoints must pass scheme, hostname, resolved-IP, redirect, and TLS policy checks before any prompt can be sent.
25. **Added WAL checkpoint, backup-safe copy, and single-instance coordination rules.** WAL/SHM files are part of durable state; backups and migrations must account for checkpoints, busy timeouts, long-lived readers, and concurrent app instances.
26. **Hardened artifact `srcdoc` base-URL behavior and preview messaging.** Previews must set a deliberate base URL, use titled iframes, validate a per-preview message channel, and move any remote/networked preview to a separate no-IPC webview.
27. **Added accessibility release gates.** Keyboard access, visible focus, labels/instructions, iframe titles, escape paths, and programmatically determinable stream/status messages are release blockers.
28. **Clarified the Tauri command-manifest requirement with a concrete command-inventory verifier.** Tauri exposes `AppManifest::commands` in current v2 documentation, but downstream agents must pin the Tauri version and add a project-owned scanner/`xtask`/CI gate that verifies `generate_handler!`, `AppManifest::commands`, capability grants, and the reviewed command inventory stay in sync.
29. **Made file intake Rust-owned by default.** JavaScript file dialogs that hand arbitrary paths to Rust are no longer treated as a sufficient security boundary; backend-issued file tokens or Rust-owned dialogs are required for attachment reads.
30. **Split artifact previews into static-preview and executable-preview risk classes.** An iframe is acceptable only for constrained preview cases; untrusted generated JavaScript must be isolated in a no-IPC surface with a hard kill path, and ideally a separate webview/process boundary.
31. **Added FTS5 query-safety requirements.** Search input must be parsed, escaped, bounded, and tested so malformed `MATCH` syntax cannot become a crash, denial-of-service path, or query injection primitive.
32. **Added provider-router transparency requirements.** “OpenRouter” is a gateway, not necessarily the final processor; the UI and request logs must expose actual routed provider/endpoint metadata when available and must not imply zero-retention or single-processor behavior without proof.
33. **Added metadata minimization rules.** `metadata_json`, provider request logs, diagnostic bundles, and support exports may not become unbounded content sinks for prompts, completions, file text, raw provider JSON, or local absolute paths.
34. **Added deletion and retention semantics.** Local deletion, backup deletion, export deletion, crash-log deletion, and provider-side retention are separate states and must be disclosed instead of collapsed into a single “deleted” label.
35. **Added app-shell remote asset and CSP hardening.** Production builds must not load remote scripts, remote styles, or CDN assets into privileged app windows; CSP must be generated and tested per window/webview.
36. **Added command-schema fuzzing and payload budget gates.** IPC commands must reject oversized, malformed, path-confused, or schema-confused payloads before they reach provider, filesystem, database, or artifact code.
37. **Added renderer failure-domain controls for previews.** A generated infinite loop must not be able to permanently freeze the host UI; reload/kill controls must be parent-owned and verified under adversarial fixtures.
38. **Added backup/export redaction policy.** Exports must enumerate exactly which fields are included and must default away from hidden metadata, provider payloads, local file paths, and content-bearing diagnostics.
39. **Added dependency-license and native-binary review gates.** Frontend and Rust dependencies that touch networking, parsing, updater, storage, crypto, or IPC must be pinned, audited, and represented in SBOM/license evidence.
40. **Added implementation-contract examples for verifier scripts.** Downstream agents must produce machine-checkable evidence, not merely prose, before claiming the spec is satisfied.
41. **Made Tauri command exposure deny-by-inventory.** Production builds must pin a Tauri v2 version that supports `tauri_build::AppManifest::commands`; the build script must enumerate allowed commands, and CI must diff that list against `tauri::generate_handler![...]`, `src-tauri/capabilities`, and `security/command-inventory.toml`. A command missing from any required list fails release.
42. **Replaced primary token streaming with Tauri IPC channels.** Primary token streaming uses `tauri::ipc::Channel<StreamEvent>`. Global Tauri events are reserved for coarse app status only. Each stream receives a backend-owned stream ID and a per-invocation channel. The frontend must ignore messages whose stream ID does not match the active stream.
43. **Required explicit release capability selection.** `tauri.conf.json` must explicitly list release capabilities. Any capability file in `src-tauri/capabilities` not selected for release must be dev-only and excluded or rejected by CI.
44. **Converted strict provider privacy from disclosure into request-time constraints.** For OpenRouter, strict mode must disable fallback routing where supported and request provider data-collection denial where supported. If the provider cannot satisfy those constraints, the request must stop before payload transmission.
45. **Banned frontend Stronghold read surfaces.** Ordinary windows must not receive Stronghold default permissions or any read, enumerate, or export operations. The only frontend-visible credential commands are domain commands such as `set_provider_key`, `delete_provider_key`, `test_provider_key`, and `get_credential_status`.
46. **Banned raw path read bridges.** Backend file-read commands may accept only backend-issued opaque file tokens. Raw JavaScript-returned paths are display hints, not read authority.
47. **Required reviewed `srcdoc` assignment.** Generated preview HTML must be assigned through a reviewed function that handles `srcdoc` escaping rules, injects the CSP/base/title wrapper, and is covered by adversarial fixtures.
48. **Expanded exact adversarial fixtures.** Release evidence must include fixture coverage for SSE errors, FTS query abuse, `srcdoc` escaping, WAL recovery, and capability drift.

---

## Adversarial Review: Blocking Design Errors Found

The previous hardened version fixed the largest errors in the original architecture, but it still left several implementation traps that would let downstream agents produce an app that looks correct while failing privacy, security, release-readiness, or real-framework feasibility requirements.

| Severity | Design Error | Why It Fails in Implementation | Required Correction |
|---|---|---|---|
| High | Tauri command exposure remains deny-by-default only on paper | Registered custom commands can be broader than the capability matrix suggests if the reviewed inventory does not explicitly restrict them. A future command can silently widen the IPC attack surface. | Tauri command exposure is deny-by-inventory. Production builds must pin a Tauri v2 version that supports `tauri_build::AppManifest::commands`. The build script must enumerate allowed commands, and CI must diff that list against `tauri::generate_handler![...]`, `src-tauri/capabilities`, and `security/command-inventory.toml`. A command missing from any required list fails release. |
| High | Stronghold can accidentally reintroduce a frontend secret read path | Using Stronghold is not automatically safe if JavaScript-accessible plugin permissions expose store-record reads to compromised UI code. | No frontend Stronghold read surface. Ordinary windows must not receive Stronghold default permissions or any read/enumerate/export operations. The only frontend-visible credential commands are domain commands such as `set_provider_key`, `delete_provider_key`, `test_provider_key`, and `get_credential_status`. |
| High | SSE parsing contract underspecifies standards-conformant event grammar | Parsing only `data:` frames can mishandle comments, multi-line data, `id`, `retry`, blank-line dispatch, and provider variants. | Implement full SSE grammar parsing before converting provider events into normalized internal stream events. |
| High | Updater authentication and private feed secrets are not governed | Private update feeds or beta channel tokens can reintroduce frontend secret paths if JavaScript supplies authorization headers or channel routing. | Keep updater credentials, private release-feed headers, channel routing, and rollback authorization Rust-owned only. |
| High | Provider debug and retention behavior is too implicit | Production debug/metadata expansion can expose transformed prompts, routing, moderation, fallback details, or content-bearing generation metadata. | Ban provider debug modes and content-bearing metadata/retrieval surfaces in production unless an explicit privacy review approves them. |
| Medium | Custom OpenAI-compatible endpoints are not governed as an SSRF surface | User-configurable endpoint URLs can pivot requests toward localhost, LAN services, private IPs, cloud metadata services, or redirect bypasses. | Enforce scheme, hostname, resolved-IP, redirect, and TLS policy before sending prompts or credentials. |
| Medium | WAL lifecycle and multi-instance coordination are incomplete | WAL/SHM files are part of persistent state; long-lived readers can block checkpoints; concurrent instances can race migrations or backups. | Add checkpoint policy, backup-safe copy rules, `busy_timeout`, and either single-instance startup or explicit multi-process migration locks. |
| Medium | Artifact preview hardening misses base-URL, assignment, and accessibility gates | `about:srcdoc` base-URL behavior, unsafe assignment, missing iframe titles, keyboard traps, invisible focus, and weak status announcements can cause security and accessibility failures. | `srcdoc` assignment must be escaped or programmatic. Generated preview HTML must be assigned through a reviewed function that handles `srcdoc` escaping rules, injects the CSP/base/title wrapper, validates a per-preview message channel, and is covered by adversarial fixtures. |
| High | Revision 3 under-specified the Tauri command-manifest mechanism | Tauri v2 documents `AppManifest::commands`, but prose alone is not enough: downstream agents can forget to pin the Tauri version, update `generate_handler!` without updating the manifest, or grant commands in capabilities that are absent from the review inventory. | Use the official `AppManifest::commands` mechanism when supported by the selected Tauri version, and add a concrete verifier: parse Rust command registration, compare against `security/command-inventory.toml`, compare manifest/capabilities against that inventory, and fail CI on drift. |
| High | Attachment intake still trusts JavaScript-controlled file paths too much | A compromised frontend can pass paths that did not come from a user dialog, can probe path existence through error behavior, or can trick backend code into reading unexpected files. | No raw path read bridge. Backend file-read commands may accept only backend-issued opaque file tokens. Raw JavaScript-returned paths are display hints, not read authority. Canonicalize and validate token-backed selections at use time. |
| High | Executable artifact previews still share too much renderer failure domain | A sandboxed iframe blocks IPC, but malicious generated JavaScript can still CPU-spin, memory-bloat, spam messages, or exploit same-webview implementation bugs. | Treat executable previews as hostile workloads. Use no-IPC separate webviews/process isolation where available, parent-owned kill/reload, CPU/message budgets, and static-preview fallback. |
| Medium | FTS5 `MATCH` syntax is not governed | User search strings can become malformed FTS queries, trigger expensive tokenization/NEAR/prefix behavior, or crash the search command path if errors are not handled. | Build FTS queries through a bounded query builder: quote terms, cap length/operator count, escape special syntax, time-box searches, and fall back to literal search on parse failure. |
| Medium | Provider-router identity remains too compressed | Saying “OpenRouter sent this” hides whether a downstream provider, fallback route, moderation layer, or retention regime actually processed the payload. | Store/display gateway provider plus final routed provider/endpoint/request ID when available. Do not imply ZDR, single-processor handling, or fixed retention without runtime metadata or account policy evidence. |
| Medium | `metadata_json` can become a privacy bypass | A schema column named metadata tends to accumulate raw provider JSON, prompt snippets, attachment text, local file paths, and diagnostic leftovers. | Define an allowlisted metadata schema with redaction tests. Reject raw request/response JSON and any field that can contain user content unless explicitly classified and export-controlled. |
| Medium | Deletion semantics are underspecified | Users may believe “delete conversation” removes backups, exports, WAL remnants, crash reports, provider records, and support bundles when it only removes active rows. | Separate active deletion, tombstone, backup purge, export purge, local vacuum/secure-delete caveat, and provider-side retention disclosure. |
| Medium | Production app-shell CSP and remote asset policy are incomplete | A privileged webview that loads CDN scripts, remote fonts, inline scripts, or broad `connect-src` expands the attack surface despite strong backend controls. | No remote scripts/styles/assets in privileged windows. Generate per-window CSP, pin allowed `connect-src`, and test release bundles for remote asset references. |
| High | Streaming still relies on broad event-bus semantics | Global Tauri events are easy for downstream agents to use because examples are common, but they blur per-invocation ownership and make wrong-stream races easier to miss. | Primary token streaming uses `tauri::ipc::Channel<StreamEvent>`. Global Tauri events are reserved for coarse app status only. Each stream receives a backend-owned stream ID and a per-invocation channel. The frontend must ignore messages whose stream ID does not match the active stream. |
| High | Release capability selection can be implicit or drift-prone | Merely having capability files in `src-tauri/capabilities` does not prove which ones are selected for packaged release, and stale dev capability files can accidentally grant authority. | Capabilities are explicitly selected. `tauri.conf.json` must explicitly list release capabilities. Any capability file in `src-tauri/capabilities` not selected for release must be dev-only and excluded or rejected by CI. |
| High | Strict privacy mode can become UI-only theater | A privacy toggle that only changes disclosure text still sends payloads through provider fallbacks or data-collection paths when the provider supports constraints but the app fails to request them. | Strict privacy mode sets provider constraints, not just UI disclosure. For OpenRouter, strict mode must disable fallback routing where supported and request provider data-collection denial where supported. If the provider cannot satisfy those constraints, the request must stop before payload transmission. |

### Non-Negotiable Security Invariants

Downstream agents must preserve these invariants even when making “small” changes:

1. **No secret read path to the frontend.** The UI may submit a credential once and query status metadata, but it may not retrieve, echo, log, export, or inspect the secret.
2. **No raw execution surface from generated content.** Generated artifacts cannot call Tauri IPC, access app storage, read credentials, execute shell commands, or silently make network requests.
3. **No arbitrary SQL, filesystem, shell, or HTTP proxy commands exposed to the frontend.** All exposed commands must be domain-specific and policy-checked.
4. **No hidden cloud boundary.** Any request that sends prompts, files, embeddings, metadata, or tool outputs off-device must be disclosed at the moment of action.
5. **No release build with development privileges.** Devtools, broad permissions, debug-only commands, permissive CSP, verbose HTTP logging, and test credentials are release blockers.
6. **No unallowlisted Tauri command surface.** Tauri command exposure is deny-by-inventory. Registered custom commands must be present in a reviewed command inventory, `tauri::generate_handler![...]`, selected release capabilities, and `tauri_build::AppManifest::commands` evidence; production builds must not enable global `window.__TAURI__` injection.
7. **No frontend-readable Stronghold vault.** Stronghold is acceptable only when ordinary UI windows receive no default, read, enumerate, export, or secret-derivation permissions and can use only domain credential commands.
8. **No unaudited model capability assumptions.** A model must be treated as unsupported until runtime metadata and request-time errors prove otherwise.
9. **No silent destructive migration.** User history must be backed up or recoverable before schema/FTS migrations mutate durable state.
10. **No unmanaged WAL state.** Backups, exports, migrations, and updates must account for WAL/SHM files, checkpoints, busy readers, and concurrent app instances.
11. **No success claim without evidence.** Every phase exit criterion must link to test output, benchmark output, artifact size reports, and security test results.
12. **No unverified framework mechanisms.** If Tauri, Svelte, SQLite, OpenRouter, or the platform provides a claimed primitive, the selected version must prove it in code and tests. If it does not, the implementation must replace the claim with a project-owned verifier, explicit code, or remove it.
13. **No arbitrary frontend path-to-read bridge.** Backend file-read commands may accept only backend-issued opaque file tokens. Raw JavaScript-returned paths are display hints, not read authority.
14. **No unbounded metadata sink.** Metadata columns, diagnostics, request logs, and support bundles must reject user-content payloads by schema, test, and review.
15. **No executable generated code in the host renderer failure domain without a hard recovery path.** Sandboxing must cover IPC, origin, network, message budget, CPU/memory abuse, and user recovery. `srcdoc` assignment must be escaped or programmatic through a reviewed wrapper.


---

## Executive Summary

The recommended architecture is a **local-history desktop AI client** built with **Tauri v2**, **Svelte 5**, **SQLite/FTS5**, and a provider abstraction capable of using **OpenRouter**, local runtimes such as **Ollama/llama.cpp**, or other OpenAI-compatible endpoints.

The app should compete with frontier web interfaces—Claude’s minimal chat focus, ChatGPT-style Canvas, and Gemini-style multimodal entry points—without inheriting Electron’s bundle weight. Tauri is still the right desktop shell because it uses the operating system’s native webview and a Rust backend. Svelte 5 remains a strong frontend choice because its rune-based reactivity gives precise UI updates and avoids a large framework runtime. SQLite remains the right default persistence layer because it is embeddable, transactional, portable, and supports FTS5 full-text search.

However, the app must **not** imply that all AI processing is local when OpenRouter is selected. It must clearly disclose that prompts, attachments, and selected conversation context are transmitted to a third-party model provider. The correct privacy promise is:

> Conversations are stored locally by default. Inference is local only when a local model provider is selected. When OpenRouter or another cloud provider is selected, the request payload leaves the device.

This distinction is not cosmetic. It determines onboarding copy, threat modeling, artifact handling, logging, retention controls, and compliance posture.

The hardened design uses the following principles:

- **Backend-only secrets:** API keys live in OS-native credential storage or a hardened secret store and are retrieved only by Rust.
- **Typed IPC:** The frontend can request operations, but cannot call arbitrary SQL, arbitrary filesystem operations, raw shell commands, or credential reads.
- **Capability-scoped windows plus deny-by-inventory command verification:** The main chat, settings, and artifact preview surfaces must have separate Tauri capabilities. Production builds must pin a Tauri v2 version that supports `tauri_build::AppManifest::commands`; CI must diff the build-script command list against `tauri::generate_handler![...]`, `src-tauri/capabilities`, and `security/command-inventory.toml`.
- **Real provider discovery:** OpenRouter model capabilities must be built from the live models endpoint and cached with an expiration policy.
- **Channel-first resilient streaming:** Primary token streaming must use `tauri::ipc::Channel<StreamEvent>`, a full SSE grammar parser, backend-owned stream IDs, cancellation handles, retry budgets, and partial-response persistence. Global Tauri events are only for coarse app status.
- **Encrypted or explicitly plaintext storage:** The architecture must choose and document whether local history is encrypted.
- **Benchmark-backed performance:** Smooth streaming is a measurable target, not a claim.

---

## Threat Model and Trust Boundaries

### Protected Assets

The implementation must protect:

- provider API keys and management keys;
- local chat history, attachments, artifacts, exports, and embeddings;
- provider request payloads, including selected context windows;
- local model endpoints and their filesystem/model paths;
- updater signing keys and release artifacts;
- SQLite database files, WAL files, backups, exports, and crash recovery files;
- user identity metadata, workspace paths, filenames, screenshots, and clipboard content.

### Adversary Classes

| Adversary | Capability | Design Response |
|---|---|---|
| Malicious generated artifact | Runs attacker-controlled HTML/CSS/JS inside preview | No Tauri IPC, no same-origin sandbox, no network by default, schema-validated messaging, runtime kill controls. |
| Compromised frontend dependency | Executes JS in the main webview | Backend-only secrets, domain-specific commands, capability scoping, backend caller validation, no arbitrary SQL/fs/shell. |
| Malicious local file | User imports crafted PDF/image/archive/Markdown/HTML | File allowlist, size/decompression limits, metadata stripping, parser isolation where practical, no automatic execution. |
| Malicious or drifting cloud provider | Changes model behavior, metadata, pricing, logging, or rate limits | Runtime discovery, disclosure, fallback, request IDs, no hardcoded promises, provider-specific error handling. |
| Local malware/same-user process | Reads files, env vars, clipboard, logs, crash dumps | OS keychain/Stronghold for secrets, optional encrypted DB, minimized logs, no plaintext key files. |
| Network attacker | Attempts MITM or endpoint substitution | HTTPS only, platform TLS validation, no user-disabled certificate checks, signed updates, explicit custom endpoint trust warnings. |
| Supply-chain attacker | Compromises npm/crate/plugin/update artifacts | Dependency pinning, lockfile review, cargo/npm audit, minimal plugins, signed release artifacts, reproducible build checks where practical. |

### Trust Boundary Diagram

```text
[User Input / Files]
        |
        v
[Svelte WebView: untrusted UI logic after dependency compromise]
        | typed invoke/events only
        v
[Tauri Runtime Authority + backend caller checks]
        |
        v
[Rust Core: secrets, DB, provider clients, migration engine]
   |        |            |
   v        v            v
[OS Keychain] [SQLite/WAL] [Cloud Provider or Local Provider]
        ^
        |
[Artifact Preview: generated code sandbox; no IPC; no same-origin]
```

The frontend is treated as a hostile-but-necessary renderer. The Rust core owns secrets, durable state, provider I/O, migrations, and policy enforcement.

### Privacy Boundary Rules

A feature is not privacy-safe merely because the history database is local. The implementation must classify every operation as one of:

| Class | Leaves Device? | Examples | Required UI Treatment |
|---|---:|---|---|
| Local-only | No | browsing local history, local search, local export | No cloud warning needed. |
| Local inference | No, unless tools are invoked | Ollama/llama.cpp on verified localhost | Show local provider badge and endpoint. |
| Cloud inference | Yes | OpenRouter request, remote custom endpoint | Show provider badge, payload disclosure, model, and attachment status. |
| Cloud metadata | Yes | model catalog refresh, provider limits query | Show in settings/privacy copy; do not include prompts. |
| Explicit export/share | User-controlled | Markdown/JSON export, save artifact | Confirmation when sensitive content may be included. |


---

## Source Confidence and Verification Policy

Downstream agents must treat all external facts by confidence tier:

| Tier | Source Type | Use In Architecture | Required Action |
|---|---|---|---|
| Tier 1 | Official docs and API responses | Normative | Cite or encode in tests |
| Tier 2 | Maintainer repositories/package docs | Acceptable with pinning | Pin version and audit dependency |
| Tier 3 | Engineering blogs | Advisory only | Verify with prototype or test |
| Tier 4 | Reddit/forums/vendor marketing | Non-normative | Do not encode as requirement without independent verification |

Volatile items that must never be hardcoded without a runtime check:

- OpenRouter free model inventory
- OpenRouter free-tier limits
- provider context windows
- provider-supported modalities
- provider-supported parameters/tool calling
- package bundle sizes
- Tauri plugin APIs
- webview behavior differences across OS versions

---

## Architectural Decision Matrices

### Table 1: Core Stack and State Management Configuration

| Component | Hardened Recommendation | Rationale and Implementation Constraint |
|---|---|---|
| Frontend framework | Svelte 5 with runes | Strong choice for small bundles and precise reactive updates. `$state`, `$derived`, and `$effect` must be used according to Svelte semantics. `$derived` must remain side-effect-free. Do not treat Svelte alone as a guarantee of 60 fps rendering. |
| Desktop framework | Tauri v2 | Strong choice for small desktop apps using the host webview and Rust commands. Must use capability-scoped permissions, a reviewed command inventory verified by CI, backend caller checks, and production-disabled `app.withGlobalTauri`. |
| Provider layer | Provider abstraction with OpenRouter as one default | OpenRouter is useful, but free-tier inventory and limits are volatile. The app must support dynamic metadata discovery, paid model fallback, and local provider fallback. |
| Data persistence | SQLite via backend repository layer | SQLite is the default store. Prefer Rust commands such as `append_message`, `search_messages`, and `delete_conversation` over exposing arbitrary frontend SQL. |
| Search | SQLite FTS5 external-content table | Use an external-content FTS5 table tied to `messages.id`, with insert/update/delete triggers and rebuild/integrity operations. |
| Secret storage | Backend-only OS keychain or Rust-owned Stronghold-backed design | The frontend must never receive the API key after initial submission. Stronghold guest/frontend read permissions must not be exposed to ordinary UI windows. The backend must retrieve secrets internally and construct provider authorization headers. |

### Table 2: Interface, User Experience, and Rendering Components

| Component | Hardened Recommendation | Rationale and Implementation Constraint |
|---|---|---|
| Markdown rendering | Use a stream-aware renderer, but prove it | `@humanspeak/svelte-markdown` may be used if pinned and tested. Do not rely on package claims alone. Long transcripts, partial code fences, tables, raw HTML, and KaTeX must be benchmarked. |
| Code/Canvas editor | CodeMirror 6 | Strong default because CodeMirror is modular and built around separate state/view packages. Only import needed languages and extensions. |
| Diff engine | Version-aware diff workflow | A diff-match-patch wrapper is acceptable only if patch state is tied to artifact revision IDs. Reject or rebase stale patches. |
| Styling | Tailwind CSS plus shadcn-svelte primitives | Good choice if bundle output is measured. Avoid monolithic component frameworks. |
| Virtualization | Required for long chat histories | The chat transcript must virtualize offscreen messages or collapse historical turns. Rendering every message forever will eventually destroy streaming performance. |

### Table 3: Security, Sandboxing, and Key Management

| Surface | Hardened Recommendation | Required Constraint |
|---|---|---|
| API key storage | OS keychain, Rust-owned Stronghold, or direct Rust `keyring` usage | No frontend read path. No API key parameter in stream commands. No localStorage/sessionStorage/plaintext config. No Stronghold store-record read permissions in ordinary UI windows. |
| Database | SQLite file, optionally encrypted | If plaintext, UI and docs must say so. If encrypted, define key lifecycle and backup/export behavior. |
| Tauri IPC | Typed, scoped commands plus reviewed command-inventory verifier | No arbitrary SQL, no arbitrary fs, no shell, no raw secret reads. Each command validates schemas and caller window label. Registered commands must be listed in the reviewed command inventory. |
| Artifact preview | Sandboxed iframe or separate no-IPC webview | `sandbox="allow-scripts"` only. Omit `allow-same-origin`. Inject a deliberate `about:srcdoc` base, title the iframe, validate the message channel, escape console output, add CSP and kill/reload controls. |
| npm supply chain | Minimize trusted frontend dependencies | Pin dependencies, run audits, avoid packages that request broad Tauri capabilities, and keep all sensitive actions in Rust. |

---

## System Architecture and Data Flow

### Privacy Boundary

The app has three privacy modes:

| Mode | Inference Location | Local Data Storage | User Disclosure |
|---|---|---|---|
| Cloud provider mode | OpenRouter or another remote endpoint | Local SQLite history | Prompts/context/attachments are sent to the provider. |
| Local provider mode | Ollama/llama.cpp/other localhost model | Local SQLite history | Prompts remain on device unless external tools are invoked. |
| Hybrid mode | User-selected per request | Local SQLite history | UI must show where each request will be processed. |

The UI must show the active provider next to the send button. Any file attachment flow must show whether the selected model/provider supports that input type and whether the file will leave the device.

### High-Performance Token Streaming: Rust to Svelte 5

Primary token streaming uses `tauri::ipc::Channel<StreamEvent>`. Global Tauri events are reserved for coarse app status only. Each stream receives a backend-owned stream ID and a per-invocation channel. The frontend must ignore messages whose stream ID does not match the active stream.

Rust backend networking still owns provider HTTP, SSE parsing, credentials, cancellation, retry control, and typed error handling. The channel is only the per-invocation delivery path for normalized `StreamEvent` values.

The frontend initiates streaming by invoking a backend command with a typed request:

```rust
#[derive(serde::Deserialize)]
struct StreamRequest {
    conversation_id: String,
    message_id: String,
    model_id: String,
    messages: Vec<ProviderMessage>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    attachments: Vec<AttachmentRef>,
}
```

The command **must not** accept an `api_key` parameter.

Correct command boundary:

```rust
#[tauri::command]
async fn stream_chat(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    request: StreamRequest,
    channel: tauri::ipc::Channel<StreamEvent>,
) -> Result<StreamStartAck, AppError> {
    state.security.assert_window_can_stream(window.label())?;

    let stream_id = StreamId::new();
    let provider = state.providers.resolve(&request.model_id).await?;
    let api_key = state.secrets.get_provider_key(provider.id()).await?;

    state
        .streams
        .start(channel, stream_id, provider, api_key, request)
        .await
}
```

The backend retrieves the provider credential internally and attaches it to the outbound HTTP request. The frontend never reads it, stores it, or receives it back.

### Real SSE Parsing Contract

The backend must implement a real Server-Sent Events parser rather than emitting arbitrary byte chunks.

Minimum requirements:

1. Check HTTP status before reading the body.
2. Capture provider request IDs and rate-limit headers where available.
3. Buffer raw bytes until complete UTF-8 text can be decoded.
4. Split the decoded stream into SSE lines while tolerating `LF` and `CRLF` endings.
5. Treat a blank line as the event-dispatch delimiter.
6. Ignore comment lines that begin with `:` without corrupting event state.
7. Parse `event`, `data`, `id`, and `retry` fields according to SSE grammar.
8. Concatenate consecutive `data:` lines for the same event with newline separators before dispatch.
9. Preserve `id`/last-event-id metadata where present and validate `retry` as an integer before using it.
10. Handle `[DONE]` or provider-specific termination markers only after a complete event has been assembled.
11. Parse provider JSON into typed deltas after SSE framing is complete.
12. Emit only normalized app events to the frontend.
13. Include `stream_id`, `conversation_id`, and `message_id` on every emitted app event.
14. Persist partial output safely on cancellation or recoverable interruption.

Recommended event types:

```typescript
type StreamEvent =
  | { type: 'token'; streamId: string; messageId: string; text: string }
  | { type: 'metadata'; streamId: string; requestId?: string; provider?: string }
  | { type: 'rate_limit_wait'; streamId: string; retryAfterMs: number }
  | { type: 'error'; streamId: string; code: string; message: string; retryable: boolean }
  | { type: 'cancelled'; streamId: string; messageId: string }
  | { type: 'done'; streamId: string; messageId: string; finishReason?: string };
```

Do not broadcast token chunks globally with `app.emit("stream-chunk", text)` or with any global Tauri event. Emit token deltas only through the per-invocation `tauri::ipc::Channel<StreamEvent>`. Global Tauri events may announce coarse app status such as catalog refresh or update availability, but not token content. The frontend must ignore channel messages whose stream ID does not match the active stream.

### Frontend RAF Throttling

Incoming token events should not mutate reactive state one event at a time. They should be buffered and flushed on `requestAnimationFrame`.

```typescript
import { Channel, invoke } from '@tauri-apps/api/core';
import { onDestroy } from 'svelte';

let activeMessage = $state('');
let activeStreamId = $state<string | null>(null);
let isStreaming = $state(false);
let tokenBuffer = '';
let animationFrameId: number | null = null;
let activeChannel: Channel<StreamEvent> | null = null;
let pendingEvents: StreamEvent[] = [];

const renderLoop = () => {
  if (tokenBuffer.length > 0) {
    activeMessage += tokenBuffer;
    tokenBuffer = '';
  }

  if (isStreaming || tokenBuffer.length > 0) {
    animationFrameId = requestAnimationFrame(renderLoop);
  } else {
    animationFrameId = null;
  }
};

const startLoop = () => {
  if (animationFrameId === null) {
    animationFrameId = requestAnimationFrame(renderLoop);
  }
};

const handleStreamEvent = (payload: StreamEvent) => {
  if (!activeStreamId) {
    pendingEvents.push(payload);
    return;
  }

  if (payload.streamId !== activeStreamId) return;

  if (payload.type === 'token') {
    tokenBuffer += payload.text;
    startLoop();
  }

  if (payload.type === 'done' || payload.type === 'cancelled' || payload.type === 'error') {
    isStreaming = false;
    startLoop();
  }
};

export async function startStream(request: StreamRequest) {
  const channel = new Channel<StreamEvent>();
  activeChannel = channel;
  activeStreamId = null;
  pendingEvents = [];
  isStreaming = true;

  channel.onmessage = handleStreamEvent;

  const ack = await invoke<StreamStartAck>('stream_chat', { request, channel });
  activeStreamId = ack.streamId;
  pendingEvents.splice(0).forEach(handleStreamEvent);
}

onDestroy(() => {
  activeChannel = null;
  pendingEvents = [];
  if (animationFrameId !== null) cancelAnimationFrame(animationFrameId);
});
```

This is a rendering strategy, not a guarantee. The implementation must still benchmark worst-case transcripts.

### Stop Generation and Cancellation

The user must be able to stop generation immediately without corrupting message state.

Backend requirements:

- Spawn each stream under a unique `stream_id`.
- Store cancellation handles in a backend-owned concurrent registry.
- Use cooperative cancellation where possible and abort handles where required.
- Persist the partial assistant message with status `cancelled` or `partial`.
- Emit a final `cancelled` event through the stream's per-invocation channel.
- Remove the stream from the active registry.

Frontend requirements:

- Disable duplicate stop clicks after cancellation starts.
- Preserve the visible partial response.
- Offer “continue from here” as a separate action.
- Never mark cancelled output as a clean completed answer.

### Retry and Backoff Semantics

OpenRouter and other providers may return `429` or `503` with a `Retry-After` header. The backend should honor it when present, but retries must be bounded and state-aware.

Rules:

1. **Before any token is emitted:** automatic retry is allowed within a retry budget.
2. **After at least one token is emitted:** do not silently replay the request. Ask the user whether to continue/regenerate because generation is not idempotent.
3. **Daily quota exhausted:** stop retrying and show provider-switch options.
4. **Network failure mid-stream:** preserve partial output and expose a clear continuation path.

### Production Build and Release-Channel Hardening

Release builds must be treated as a separate security target, not as “dev build plus packaging.”

Required release controls:

Release capability selection is a release blocker. `tauri.conf.json` must explicitly list the release capability set rather than relying on directory presence or broad defaults. CI must enumerate every file in `src-tauri/capabilities`, classify it as release-selected or dev-only, and fail if an unselected capability can be packaged into release or if a selected capability grants a command not present in the reviewed inventory.

| Control | Requirement |
|---|---|
| Devtools | Disabled by default in production. Any emergency devtools build must have a separate signed channel and visible watermark. |
| CSP | Production CSP must be explicit and restrictive. Any `unsafe-inline` exception must be limited to sandboxed artifact documents, not the host app. |
| Source maps | Do not ship public source maps that expose local paths, secrets, or internal command names unless explicitly reviewed. |
| Debug commands | Compile out or hard-deny test commands, diagnostic secret dumps, SQL consoles, fixture loaders, and mock provider endpoints. |
| Capabilities | Capabilities are explicitly selected. `tauri.conf.json` must explicitly list release capabilities. Any capability file in `src-tauri/capabilities` not selected for release must be dev-only and excluded or rejected by CI. Registered custom commands must also be checked by the command-inventory verifier. |
| Global Tauri object | `app.withGlobalTauri` must be disabled in production so frontend code cannot rely on global `window.__TAURI__` injection. |
| Logging | Default log level must not include HTTP bodies, headers, authorization tokens, prompts, attachments, or generated artifact contents. |
| Provider debug | Provider debug modes, metadata-expansion headers, and content-bearing generation retrieval features must be disabled in production unless a documented privacy review explicitly approves them. |
| Update channel | Stable, beta, and internal builds must use separate update manifests and signing keys or clearly separated signing identities. |
| Artifact verification | CI must publish checksums, SBOM/license report, size report, and signature verification evidence. |

### Application Shell CSP and Remote Asset Policy

The host application webviews are privileged surfaces even when individual commands are locked down. A compromised remote script loaded into the main window is still a compromised renderer.

Rules:

- Production privileged windows must not load JavaScript, CSS, fonts, images, Markdown renderers, syntax highlighters, analytics snippets, or model-provider widgets from CDNs or arbitrary remote URLs. Bundle and pin frontend assets at build time.
- The main app CSP must be generated per window/webview and tested in packaged release mode, not only during dev-server mode.
- `connect-src` must be narrow. The main app should normally connect only to the Tauri IPC bridge and explicitly approved provider/update endpoints owned by Rust-side clients. Provider API calls should be made from Rust, not from frontend `fetch`.
- Any exception for remote images in rendered Markdown must use a privacy-preserving proxy or explicit user action. Auto-loading remote images leaks IP address, timing, and conversation context.
- Do not permit `unsafe-eval` in privileged windows. Any `unsafe-inline` exception must be justified, scoped, and tested; artifact `srcdoc` CSP is separate from the app-shell CSP.
- Release tests must scan built HTML/CSS/JS for `http://`, `https://`, CDN hostnames, source-map references, and unexpected inline script allowances.

### Updater and Distribution Threat Model

If the app supports automatic updates, the updater becomes part of the security boundary.

Minimum requirements:

1. Use signed update artifacts and verify signatures before install.
2. Do not fetch update manifests from user-editable or insecure locations.
3. Pin the update channel selected by the installed build.
4. Keep private update-feed URLs, authorization headers, bearer tokens, beta-channel secrets, and channel-routing decisions in Rust-owned code/configuration only.
5. Reject any JavaScript-supplied updater credential, private-feed header, or channel override.
6. Prevent downgrade attacks unless the rollback is explicitly signed, Rust-authorized, and user-confirmed.
7. Fail closed on signature/manifest/authentication errors.
8. Keep updater logs free of secrets, prompts, file paths, and private feed URLs.
9. Test interrupted updates and partial downloads.
10. Document key/channel rotation and manual recovery for failed updates.

No release candidate is acceptable until update verification has been tested on Windows, macOS, and Linux packaging targets.


---

## Security and Privacy Implementation Guide


### Logging, Telemetry, and Crash-Report Policy

Logging is a privacy boundary. A correct credential design can still fail if logs capture sensitive values.

Rejected log content:

- authorization headers, API keys, key fingerprints beyond intentional `lastFour`;
- prompt bodies, assistant bodies, tool outputs, embeddings, files, image/PDF base64, or generated artifact bodies;
- full provider request/response JSON;
- local absolute paths unless user opts into diagnostic export;
- clipboard contents;
- raw SQL values containing message content;
- panic payloads that include request structs.

Allowed log content:

- event type;
- stream ID and message public ID;
- provider ID and model ID;
- HTTP status code;
- provider request ID;
- byte counts and timing;
- redacted error code;
- benchmark counters.

Required tests:

```text
grep release logs for:
- sk-or-
- Bearer
- Authorization
- api_key
- prompt content fixture strings
- attachment fixture strings
- local username/home path fixture strings
```

Crash reporting must be opt-in. Crash reports must be scrubbed before upload and must never include database files, WAL files, attachments, exported conversations, credentials, or request bodies.

### Provider Debug, Metadata, and Retention Policy

Provider observability is a privacy boundary. Production builds must assume that provider debug surfaces, expanded generation metadata, fallback-routing details, moderation metadata, and content-bearing generation retrieval APIs can expose transformed prompts, completions, or derived sensitive content.

Normative rules:

- Provider `debug` modes are banned in production by default.
- Metadata-expansion headers, request replay features, stored-generation retrieval, and prompt/completion body retrieval are banned in production unless a documented privacy review explicitly allows a narrowly scoped diagnostic build.
- Diagnostic builds that enable provider debug behavior must use a separate signed channel, a visible watermark, opt-in user consent, and automatic expiry.
- The request log may store provider ID, model ID, status, usage, timing, retry classification, and provider request ID, but it must not store prompt bodies, completion bodies, attachment bodies, or raw provider JSON.
- Any provider-side retention behavior that is not controlled by the app must be disclosed in privacy copy as provider-governed rather than local-only.
- Support export must default to metadata-only diagnostics and require a second confirmation before including user-visible message text.

### Attachment and File-Ingestion Security

Attachment support is a high-risk feature and must not be implemented as “read file and send base64.”

Required ingestion pipeline:

1. No raw path read bridge. Backend file-read commands may accept only backend-issued opaque file tokens. Raw JavaScript-returned paths are display hints, not read authority.
2. Prefer a Rust-owned file dialog command that mints a backend-owned `FileSelectionToken` or `OpaqueFileToken`. JavaScript may display selected filenames, but it must not be able to cause arbitrary backend reads by inventing paths.
3. If the JavaScript dialog plugin is used for UX reasons, the backend must still treat the returned path as untrusted display data and must require a fresh user-selection proof, capability scope, or token minted by a Rust command.
4. Backend canonicalizes the selected path at use time and rejects traversal, symlink surprises where relevant, device files, hidden unsupported bundles, network paths, UNC paths, app-private database paths, keychain/Stronghold snapshots, and updater/signing material unless a specific feature explicitly allows them.
5. Backend validates size before reading and should use streaming reads for anything non-trivial.
6. Backend validates type using extension plus content sniffing.
7. Backend rejects archives by default. If archives are later supported, enforce decompression ratio, file count, nested path, and total expanded-size limits.
8. Backend strips or warns about metadata where practical. Images should have EXIF stripped before cloud upload unless the user explicitly chooses to preserve metadata.
9. UI shows whether the selected provider/model supports the attachment and whether the attachment will leave the device.
10. UI requires explicit confirmation before sending files to a cloud provider or non-loopback custom endpoint.
11. Backend records attachment metadata separately from message text and supports deletion/export policy. It should store sanitized display names and hashes, not absolute source paths, unless the user explicitly enables path-preserving diagnostics.

Baseline attachment budgets:

| Input Type | Default Max | Notes |
|---|---:|---|
| Plain text/Markdown/code | 2 MB | Larger files require chunking/indexing flow. |
| PDF | 25 MB | Parse/extract locally when possible before provider upload. |
| Image | 10 MB | Strip EXIF where possible before cloud send. |
| Archive | Rejected | Add only in a later threat-modeled feature. |
| HTML/SVG | Treat as hostile text | Never render in main DOM. Preview only through artifact sandbox. |

Path and metadata handling:

| Field | Default Handling | Rationale |
|---|---|---|
| Original filename | Store sanitized basename only | Full paths reveal usernames, project names, and private folder structure. |
| Absolute source path | Do not persist by default | Needed transiently for reading, but dangerous in logs/exports. |
| File hash | Store SHA-256 for dedupe/integrity | Safe if not used as a public identifier for known private files. |
| Extracted text | Store only if user consents to local indexing | Extracted text can be as sensitive as the original file. |
| EXIF/document metadata | Strip before cloud send where practical | Metadata may contain GPS, author, app, path, and device identity. |

### Clipboard and Export Safety

Generated artifacts and conversations often leave the app through copy/export.

Rules:

- Copy buttons must copy the selected finalized content, not hidden prompt metadata.
- Export must show included fields: messages, system prompts, attachments, model IDs, timestamps, provider request IDs, artifacts, and metadata.
- HTML export must escape content by default.
- Markdown export must not include executable frontmatter or hidden HTML unless user opts in.
- JSON export must not include provider API keys, secret status internals, local database keys, or updater metadata.
- “Delete conversation” must disclose whether backups, exports, logs, or provider-side data may still exist.


### API Key Storage: Backend-Only Secret Retrieval

The frontend may collect an API key during onboarding and submit it once to a write-only backend command:

```rust
#[tauri::command]
async fn set_provider_key(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    provider_id: String,
    api_key: SecretString,
) -> Result<(), AppError> {
    state.security.assert_window_can_manage_settings(window.label())?;
    state.secrets.store_provider_key(&provider_id, api_key).await
}
```

There must be no `get_provider_key` frontend command. The UI can ask whether a key exists, but the backend should return only metadata:

```typescript
type CredentialStatus = {
  providerId: string;
  configured: boolean;
  lastFour?: string;
  createdAt?: string;
};
```

Acceptable storage options:

| Option | Status | Notes |
|---|---|---|
| Direct Rust `keyring` crate | Preferred simple option | Keeps secret API in backend. Requires platform testing. |
| Rust-owned Tauri Stronghold vault | Preferred for encrypted app vaults | More UX complexity. Must initialize securely. Ordinary UI windows must not receive Stronghold record-read permissions. |
| Community keyring plugin | Conditional | Only after supply-chain review and only if frontend read commands are disabled, not exposed, and covered by capability tests. |
| Frontend-accessible Stronghold store-record reads | Rejected | A compromised webview could read secrets directly and violate the backend-only secret invariant. |
| localStorage/sessionStorage/plaintext config | Rejected | Violates baseline security promise. |


### Stronghold-Specific Constraints

If Stronghold is used, it must be treated as an encrypted backend vault, not as a general JavaScript-accessible application store.

Rules:

- No frontend Stronghold read surface is allowed. Ordinary windows must not receive Stronghold default permissions or any plugin permission, procedure, or command path that can read, enumerate, export, or derive provider keys, database keys, updater feed credentials, or signing/channel secrets.
- The only frontend-visible credential commands are domain commands such as `set_provider_key`, `delete_provider_key`, `test_provider_key`, and `get_credential_status`. No `get_provider_key`, raw Stronghold read, enumerate, export, snapshot, or derived-secret command may exist in a release capability.
- Stronghold vault names, record paths, snapshot paths, and key-derivation inputs must not be exposed through frontend-visible errors or diagnostics.
- Stronghold unlock/initialization failures must fail closed and must not fall back to plaintext storage.
- Tests must prove that a compromised main webview cannot read, enumerate, export, or copy Stronghold records.

### Secret Lifecycle Requirements

Secret storage is not complete until lifecycle behavior is defined.

| Operation | Required Behavior |
|---|---|
| Add key | Accept once through settings command; validate format only enough to prevent obvious mistakes; never echo full value. |
| Test key | Send a minimal provider metadata/auth check that contains no prompts. |
| Rotate key | Store new key atomically; invalidate active provider clients using the old key. |
| Delete key | Remove from keychain/Stronghold and memory caches; show provider as unconfigured. |
| Export settings | Export provider IDs and preferences, never secrets. |
| Crash/panic | Secret values must not appear in formatted errors or debug output. |
| Memory | Use secret wrappers where practical and avoid cloning key strings into long-lived frontend-visible structures. |


### Local History Database Privacy

The product must choose one of two explicit storage modes.

#### Option A: Plain SQLite History

Allowed only if the UI and documentation say:

> Chat history is stored locally in a SQLite database protected by normal operating-system user permissions. It is not encrypted by default.

This option is easier to back up and debug but weaker against local malware, another process running as the same user, stolen disks without full-disk encryption, and forensic recovery.

#### Option B: Encrypted Local History

Recommended for privacy-forward positioning.

Requirements:

- Generate a random database encryption key on first launch.
- Store the database key in OS keychain or Stronghold.
- Support export to plaintext Markdown/JSON only after explicit user action.
- Document that deleting a conversation may not securely erase historical bytes unless secure-delete/vacuum strategy is implemented.
- Test backup/restore across OSes.

Do not claim “local-first privacy” if using plaintext storage and cloud inference without clear disclosure.

### Deletion, Retention, and Erasure Semantics

Deletion must be precise. A user-facing delete action cannot honestly imply provider-side erasure, backup erasure, WAL erasure, crash-report erasure, or export erasure unless each surface is actually handled.

Required states:

| State | Meaning | User Copy Requirement |
|---|---|---|
| Active | Visible in normal history | Normal history/search. |
| Locally deleted | Removed from active tables and FTS | Explain whether local backups/exports may still contain it. |
| Tombstoned | Deletion marker retained for sync/audit | Do not show content; retain only minimal deletion metadata. |
| Backup retained | Still present in local backup/snapshot | Backup purge must be a separate action or scheduled policy. |
| Exported | User-created external copy may exist | App cannot revoke external files. |
| Provider-retained | Cloud provider may retain/process under its own policy | Disclose provider-governed retention; do not promise remote deletion unless API evidence exists. |
| Secure-erased | Best-effort local vacuum/secure-delete flow completed | State limitations for SSDs, OS caches, and previous backups. |

Implementation rules:

- Delete must update FTS and attachment indexes in the same transaction where possible.
- Deleting an attachment must remove or tombstone extracted text and thumbnails, not only the binary file.
- Backup purge must understand WAL/SHM and checkpoint state.
- Support export must mark whether deleted/tombstoned records are excluded.
- The privacy UI must avoid a single ambiguous “forever deleted” claim.


### Tauri IPC Capability Contract

Every window/webview must have an explicit capability profile, and every registered custom command must also be explicitly present in a reviewed command inventory. Capability files constrain who may call a command; the verifier must constrain which commands may be registered in the compiled app at all.

| Window/WebView | Allowed Commands | Denied Commands | Allowed Plugins |
|---|---|---|---|
| `main` chat window | `stream_chat`, `cancel_stream`, `list_conversations`, `append_message`, `search_messages`, `get_model_catalog`, `get_credential_status` | raw filesystem, shell, arbitrary SQL, read secret, delete credential unless confirmed | events only, limited invoke |
| `settings` window/pane | `set_provider_key`, `delete_provider_key`, `get_credential_status`, provider configuration commands | stream commands, arbitrary SQL, artifact execution | credential write/delete only |
| `artifact-preview` iframe/webview | none | all Tauri IPC | none |
| `debug`/developer tools window, if any | read-only diagnostics | secrets, arbitrary SQL, shell | diagnostics only |

Backend commands must validate the caller’s window label in addition to Tauri capability files. Capability configuration is necessary but not sufficient.

Command-inventory verifier requirements:

- Tauri command exposure is deny-by-inventory. Production builds must pin a Tauri v2 version that supports `tauri_build::AppManifest::commands`. The build script must enumerate allowed commands, and CI must diff that list against `tauri::generate_handler![...]`, `src-tauri/capabilities`, and `security/command-inventory.toml`. A command missing from any required list fails release.
- Maintain `security/command-inventory.toml` or an equivalent source-controlled review file that lists each custom command, owning module, allowed window labels, production/debug status, argument schema, sensitivity class, and expected capability grant.
- Use `tauri_build::AppManifest::commands` or the selected Tauri version's equivalent compiled-command allowlist where available. Also implement a project-owned verifier, for example `cargo xtask verify-command-inventory`, that parses the Rust command registration site such as `tauri::generate_handler![...]`, compares it with the inventory, selected release capabilities, and manifest, and fails CI on missing, extra, or debug-only commands.
- Fail CI if a release capability grants a command not present in the reviewed inventory, if an inventory command is absent from `tauri::generate_handler![...]`, or if the build-script/AppManifest command enumeration drifts from either source.
- Fail CI if an inventory command lacks tests for wrong-window invocation, malformed payloads, oversized payloads, and release/debug behavior.
- Keep debug, fixture, SQL-console, secret-inspection, mock-provider, and updater-test commands compiled out or hard-denied in production.
- Keep `app.withGlobalTauri` disabled in production; frontend code must import the specific Tauri APIs it needs rather than relying on `window.__TAURI__`.
- Treat plugin permissions the same way as custom commands: least privilege, per-window, release-tested, and represented in the inventory.

Example inventory shape:

```toml
[[command]]
name = "stream_chat"
module = "chat::commands"
windows = ["main"]
production = true
sensitivity = "cloud_payload"
max_payload_bytes = 1_000_000
requires_runtime_schema_test = true

[[command]]
name = "set_provider_key"
module = "settings::commands"
windows = ["settings"]
production = true
sensitivity = "secret_write_only"
max_payload_bytes = 20_000
requires_wrong_window_test = true
```

### Isolation Pattern

Tauri’s Isolation Pattern can be used as defense-in-depth. It should not be treated as a magic cryptographic shield. The implementation still needs least-privilege command design, typed schemas, capability scoping, and backend-side validation.

Use the Isolation Pattern only if:

- the team accepts the extra complexity and runtime overhead;
- the secure isolation app has a minimal dependency tree;
- all IPC payloads are schema-validated;
- the security boundary is documented and tested.

Do not use Isolation as a substitute for proper command design.

### Artifact Sandboxing and XSS Mitigation

Interactive HTML/CSS/JS previews are high risk. AI-generated code must never be rendered directly into the main app DOM. Revision 5 preserves the split between **static preview** and **executable preview**. Static HTML preview can use a sandboxed iframe. Executable JavaScript preview should use a separate no-IPC webview/window or the strongest available renderer isolation, because a same-renderer iframe can still cause denial-of-service through CPU loops, memory abuse, or message floods.

Minimum iframe sandbox for static or constrained preview:

```html
<iframe
  title="Generated artifact preview"
  sandbox="allow-scripts"
  srcdoc="...">
</iframe>
```

Do not include `allow-same-origin`.


`srcdoc` assignment must be escaped or programmatic. Generated preview HTML must be assigned through a reviewed function that handles `srcdoc` escaping rules, injects the CSP/base/title wrapper, and is covered by adversarial fixtures. Downstream code must not concatenate arbitrary generated HTML directly into an iframe attribute or assign unwrapped HTML directly to `iframe.srcdoc`.

Required wrapper behavior:

```typescript
function buildArtifactSrcdoc(input: GeneratedPreviewHtml): string {
  const escapedOrParsed = normalizeGeneratedPreviewHtml(input);
  return renderReviewedSrcdoc({
    title: 'Generated artifact preview',
    baseHref: 'about:srcdoc',
    csp: "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; connect-src 'none'; font-src data:; base-uri 'none'; form-action 'none';",
    body: escapedOrParsed
  });
}
```

Adversarial fixtures must cover quote breaking, `</iframe>` injection, malformed `<base>`, CSP bypass attempts, event-handler attributes, SVG/script payloads, huge inline payloads, Unicode confusables, and relative URL resolution.

The `srcdoc` document must set a deliberate base URL and include a restrictive CSP before generated content. Example baseline:

```html
<base href="about:srcdoc">
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; connect-src 'none'; font-src data:; base-uri 'none'; form-action 'none';">
```

The `<base href="about:srcdoc">` requirement prevents generated relative URLs from resolving against the embedding app document. The iframe `title` is mandatory for assistive technology.

If networked previews or generated JavaScript execution are ever allowed, that must be a separate explicit user permission and not the default. Remote, network-capable, or executable previews must run in a separate no-IPC webview/window, not in a webview that also has access to application commands. The parent window must own reload/kill controls that do not depend on cooperation from the generated code.

`postMessage` console bridge requirements:

- Check `event.source === iframe.contentWindow`.
- Use a per-preview random `previewSessionId` and rotate it on every reload.
- Prefer a narrow `MessageChannel`/one-way console bridge when practical; otherwise reject all messages that do not match the exact schema.
- Validate message schema with a runtime validator.
- Limit payload size and message frequency per preview.
- Rate-limit console messages and collapse repeated output.
- Escape all output rendered in the host console.
- Ignore unknown message types and unknown session IDs.
- Provide a keyboard-accessible kill/reload control for infinite loops.
- Provide an escape path that returns focus from the preview to the host app.
- Enforce a maximum console payload size, message count per second, and total retained console bytes.
- Treat preview reload as a new trust boundary: rotate `previewSessionId`, clear message channels, and discard old console buffers.
- Verify that an infinite loop, recursive DOM mutation, large allocation, or message flood cannot permanently freeze the host UI.

Example parent-side handling:

```typescript
function handlePreviewMessage(event: MessageEvent) {
  if (event.source !== iframeRef?.contentWindow) return;

  const parsed = PreviewConsoleMessage.safeParse(event.data);
  if (!parsed.success) return;
  if (parsed.data.previewSessionId !== currentPreviewSessionId) return;

  appendEscapedConsoleEntry(parsed.data);
}
```

---

## Data Persistence and Fast Search Implementation

### Persistence Boundary

Persistence should be owned by the Rust backend. The frontend asks for domain operations; it does not execute arbitrary SQL.

Recommended repository commands:

```text
create_conversation(title?)
list_conversations(cursor, limit)
read_conversation(conversation_id)
append_message(conversation_id, role, content, metadata)
update_message(message_id, content, status)
delete_conversation(conversation_id)
search_messages(query, filters, cursor, limit)
export_conversation(conversation_id, format)
```

### Core Schema

Recommended baseline:

```sql
CREATE TABLE conversations (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  archived_at TEXT,
  provider_id TEXT,
  model_id TEXT
);

CREATE TABLE messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  public_id TEXT NOT NULL UNIQUE,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('system', 'user', 'assistant', 'tool')),
  content TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'complete' CHECK (status IN ('streaming', 'complete', 'partial', 'cancelled', 'error')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  token_count INTEGER,
  provider_request_id TEXT,
  metadata_json TEXT
);

CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_conversations_updated_at ON conversations(updated_at DESC);
```

Use integer `messages.id` as the FTS rowid. Expose `public_id` to the frontend if needed.

### Metadata Schema and Content-Sink Policy

`metadata_json` is not a dumping ground. It must be an allowlisted schema with explicit field classes.

Allowed examples:

```json
{
  "schema_version": 1,
  "client_revision": "artifact-rev-id",
  "visible_status": "partial",
  "provider": { "id": "openrouter", "routed_provider": "unknown", "request_id": "redacted-or-provider-id" },
  "usage": { "prompt_tokens": 1234, "completion_tokens": 456 }
}
```

Rejected metadata content:

- raw provider request or response JSON;
- prompt or completion text;
- extracted attachment text;
- base64 files or images;
- local absolute paths;
- authorization headers, cookies, API-key fragments beyond an intentional display-only `lastFour`;
- hidden system prompts or private routing instructions that exports would miss.

Rules:

1. Validate metadata through a typed schema before database write.
2. Put content-bearing data in explicit content tables with deletion/export policy, not hidden metadata.
3. Support export profiles: user-visible content only, metadata-only diagnostics, and full local backup.
4. Add fixtures whose secret strings appear only in message content and prove they do not leak into metadata, logs, request logs, or support bundles.

### FTS5 External-Content Schema

Use an external-content FTS5 table:

```sql
CREATE VIRTUAL TABLE messages_fts USING fts5(
  content,
  conversation_id UNINDEXED,
  role UNINDEXED,
  content='messages',
  content_rowid='id',
  tokenize='unicode61'
);
```

Synchronization triggers:

```sql
CREATE TRIGGER messages_ai AFTER INSERT ON messages BEGIN
  INSERT INTO messages_fts(rowid, content, conversation_id, role)
  VALUES (new.id, new.content, new.conversation_id, new.role);
END;

CREATE TRIGGER messages_ad AFTER DELETE ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id, role)
  VALUES ('delete', old.id, old.content, old.conversation_id, old.role);
END;

CREATE TRIGGER messages_au AFTER UPDATE OF content, conversation_id, role ON messages BEGIN
  INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id, role)
  VALUES ('delete', old.id, old.content, old.conversation_id, old.role);

  INSERT INTO messages_fts(rowid, content, conversation_id, role)
  VALUES (new.id, new.content, new.conversation_id, new.role);
END;
```

Migration requirement:

```sql
INSERT INTO messages_fts(messages_fts) VALUES('rebuild');
```

Add an integrity check task in diagnostics:

```sql
INSERT INTO messages_fts(messages_fts, rank) VALUES('integrity-check', 1);
```

### FTS5 Query Safety

Search is an input parser, not a string concatenation site. FTS5 `MATCH` has its own query language; malformed or hostile input must not become crashes, unbounded scans, or confusing query behavior.

Required query builder behavior:

- Trim and normalize Unicode input.
- Cap query byte length, token count, prefix wildcard count, phrase length, and boolean/operator count.
- Escape user terms and quote phrases by default.
- Treat advanced FTS syntax as an explicit “advanced search” mode, not the default.
- Catch SQLite/FTS parse errors and fall back to a literal term search or a clear validation message.
- Apply result limits and pagination.
- Time-box search commands or run them on a worker thread so a pathological query cannot freeze streaming or UI.
- Test inputs containing quotes, `NEAR`, `*`, `-`, column filters, unmatched parentheses, zero-width characters, extremely long tokens, and mixed scripts.

Rejected pattern:

```rust
// Rejected: user input becomes raw MATCH syntax.
sqlx::query("SELECT rowid FROM messages_fts WHERE messages_fts MATCH ?")
  .bind(user_supplied_query);
```

Acceptable pattern:

```rust
let safe_query = fts_query_builder::literal_terms(user_supplied_query, QueryBudget::default())?;
repo.search_messages_fts(safe_query, page).await?;
```

### WAL and Write Strategy

Use WAL mode for normal chat operations:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL; -- use FULL for migrations/checkpoints where durability risk is unacceptable
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

The `-wal` and `-shm` files are part of the database's durable runtime state. Backup, export, migration, updater, and crash-recovery code must not treat the main `.sqlite` file as the whole database while WAL is active.

Checkpoint policy:

- Use SQLite's backup API for live backups where possible instead of copying files directly.
- Before file-copy backup/export, perform a controlled checkpoint such as `PRAGMA wal_checkpoint(TRUNCATE);` and verify that no long-lived read transaction blocked the checkpoint.
- Run a passive checkpoint periodically after large streaming/import sessions to prevent unbounded WAL growth.
- Treat checkpoint failure as a recoverable maintenance warning unless it happens during backup/export/migration, where it becomes a blocking error.
- Include WAL/SHM fixtures in migration and restore tests.

Do not write every token to SQLite. The backend or frontend should buffer streaming output and persist at controlled intervals or on completion/cancellation. Suggested policy:

- persist draft assistant row at stream start with status `streaming`;
- update every 1-3 seconds or every N characters, whichever comes later;
- finalize with status `complete`, `partial`, `cancelled`, or `error`.

### Migration, Backup, and Recovery Policy

SQLite migrations are dangerous because user history is durable product value. Treat migrations as data-preservation operations, not schema chores.

Required migration behavior:

1. Maintain a `schema_migrations` table with id, checksum, applied_at, app_version, and success/failure status.
2. Run migrations inside transactions where SQLite permits.
3. Before destructive migrations, create a user-recoverable backup or snapshot.
4. On startup, detect dirty or partial migrations and enter recovery mode instead of continuing normally.
5. Rebuild FTS indexes after migrations that affect indexed columns.
6. Run `PRAGMA integrity_check` and FTS5 integrity checks after high-risk migrations.
7. Test migrations against:
   - empty database;
   - current production schema;
   - one-version-old schema;
   - database with WAL and SHM files;
   - database interrupted mid-stream;
   - database with inconsistent FTS index;
   - database with large history and attachments.
8. Store migration fixtures under version control, not generated ad hoc.
9. Enforce single-instance startup before migrations where practical, or acquire an explicit cross-process migration lock before mutating schema/state.
10. Block app updates, destructive maintenance, and backup/export operations while another instance or long-running migration is active.

### Single-Instance and Migration Coordination

The preferred desktop behavior is a single running app instance. If the implementation uses Tauri's single-instance plugin, register it before plugins or setup code that can touch the database, updater, credential store, or filesystem. If multi-instance behavior is intentionally supported, the design must include an explicit cross-process lock around migrations, WAL checkpoints, backup/export, destructive deletes, and updater-triggered restarts.

Required coordination behavior:

- A second app launch should focus or signal the first instance rather than racing database startup.
- Migration startup must fail closed if another process holds the migration lock.
- Backup/export must either use SQLite's backup API or checkpoint safely under lock.
- Update install/restart must not run while a migration or backup/export is active.
- Diagnostics must report active locks, checkpoint status, and recovery mode clearly.

Recommended tables not present in the minimal core schema:

```sql
CREATE TABLE schema_migrations (
  id TEXT PRIMARY KEY,
  checksum TEXT NOT NULL,
  app_version TEXT NOT NULL,
  applied_at TEXT NOT NULL,
  success INTEGER NOT NULL CHECK (success IN (0, 1))
);

CREATE TABLE attachments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  public_id TEXT NOT NULL UNIQUE,
  message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  original_name TEXT NOT NULL,
  media_type TEXT NOT NULL,
  byte_size INTEGER NOT NULL,
  sha256 TEXT NOT NULL,
  storage_path TEXT,
  redaction_status TEXT NOT NULL DEFAULT 'none',
  created_at TEXT NOT NULL
);

CREATE TABLE provider_request_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  message_id INTEGER REFERENCES messages(id) ON DELETE SET NULL,
  provider_id TEXT NOT NULL,
  model_id TEXT NOT NULL,
  provider_request_id TEXT,
  http_status INTEGER,
  error_code TEXT,
  prompt_token_count INTEGER,
  completion_token_count INTEGER,
  created_at TEXT NOT NULL
);
```

The request log must not contain prompt or completion bodies.


---

## UI/UX Patterns and Component Architecture

### Svelte 5 Runes Integration

Use runes deliberately:

- `$state` for mutable UI state: active input, active stream ID, open panels, selected provider/model.
- `$derived` for pure derived values: selected model capability, context budget warning, filtered conversation lists.
- `$effect` for lifecycle-side effects such as event listeners, persistence triggers, or scroll behavior.

Do not put side effects inside `$derived`.

### Chat Transcript Rendering

The chat transcript must be designed for long-running use.

Required features:

- virtualized message list or collapsed historical sections;
- incremental rendering of active assistant output;
- deferred syntax highlighting for incomplete code fences;
- deferred KaTeX rendering until block boundaries are stable;
- copy buttons that operate on finalized message content, not raw in-flight chunks;
- visible and programmatically determinable status for streaming, partial, cancelled, and error outputs;
- keyboard-accessible message actions with visible focus indicators.

### Accessibility Release Baseline

Accessibility is a release gate, not a polish task.

Minimum requirements:

- All interactive controls must be keyboard reachable without pointer input.
- Focus order must match the visual/task order of the chat, sidebar, composer, canvas, and preview controls.
- Focus indicators must be visible in normal and high-contrast themes.
- Form fields, model selectors, attachment controls, destructive actions, and settings must have programmatically associated labels or instructions.
- Streaming, cancellation, retry, provider-switch, upload, export, and error states must be announced through appropriate status/live regions without interrupting normal typing.
- Artifact iframes must have meaningful titles and a keyboard-accessible escape path back to the host UI.
- The preview kill/reload control must be reachable by keyboard and must work even while generated code is misbehaving.
- Release testing must include keyboard-only operation for core chat, provider settings, attachment send, artifact preview, and export flows.

### Markdown and Mathematics Rendering

`@humanspeak/svelte-markdown` may remain the candidate renderer, but the implementation must prove it under adversarial streaming fixtures.

Required fixtures:

1. Unclosed Markdown code fence streamed over multiple chunks.
2. Unclosed HTML tag split across chunks.
3. Nested Markdown table streamed cell-by-cell.
4. KaTeX block with delayed closing delimiter.
5. Raw SVG/HTML attempt with script injection.
6. Long list with 1,000 items.
7. Markdown containing links, images, and malformed nesting.

Any renderer that allows script execution in the main app DOM is rejected.

### Split-Pane Canvas / Artifacts Mode

The Canvas workspace uses a split-pane layout:

- left pane: conversation and instructions;
- right pane: artifact editor, preview, and patch review.

CodeMirror 6 remains the recommended editor. Import only required extensions:

```typescript
import { EditorView } from '@codemirror/view';
import { EditorState } from '@codemirror/state';
import { javascript } from '@codemirror/lang-javascript';
import { markdown } from '@codemirror/lang-markdown';
```

Do not import a full language suite by default.

### Versioned Artifact State

Every artifact must be versioned.

```text
artifact_id
current_revision_id
base_revision_id
editor_doc
dirty_since_revision
pending_patch_id
patch_target_revision_id
accepted_patch_ids
rejected_patch_ids
```

Patch application rules:

1. When the user asks the AI to edit an artifact, record the current revision as `patch_target_revision_id`.
2. If the user edits the artifact while the AI patch is streaming, mark the patch as stale.
3. Stale patches must be rejected or explicitly rebased.
4. Accepted patches create a new revision.
5. Rejected patches are retained in metadata for audit/history but not applied.

This prevents AI patches from silently overwriting user work.

---

## Provider Routing and Rate Limit Handling

### Dynamic Capability Matrix

The application must build its model catalog dynamically.

Minimum model metadata:

```typescript
type ModelCapability = {
  id: string;
  name?: string;
  provider?: string;
  contextLength?: number;
  inputModalities: string[];
  outputModalities: string[];
  supportedParameters: string[];
  isFreeVariant: boolean;
  pricing?: unknown;
  fetchedAt: string;
};
```

Rules:

- Filter free models by IDs ending in `:free` or official metadata, not by a static list.
- Hide or warn on models that lack required modalities.
- Disable image/file controls unless the selected model explicitly supports them.
- Warn when conversation context may exceed selected model limits.
- Cache model metadata with a TTL and manual refresh.
- Degrade gracefully if model metadata fetch fails.

### Provider Drift and Runtime Revalidation

The model catalog is an input to the UI, not a promise. Provider behavior may drift between catalog refresh and send time.

Rules:

- Cache catalog responses with `fetched_at`, `expires_at`, provider version/etag if available, and raw capability hash.
- Revalidate stale model metadata before file or multimodal sends.
- If the provider rejects a request for unsupported modality, context length, parameter, or model availability, update local metadata and show a precise failure.
- Do not infer support for tools, JSON schema, PDFs, images, or reasoning parameters from provider family names.
- Treat custom endpoints as untrusted until health checks confirm API shape and endpoint egress policy passes.
- Record provider request IDs for user support without logging bodies.
- Keep provider debug modes and metadata-expansion behavior default-off, and forbid them in production without privacy review.
- Classify provider surfaces as metadata-only, usage-only, or content-bearing before enabling diagnostics or support export.
- Provide a “dry run capability check” where possible for settings diagnostics.

### Provider Router and Downstream Processor Transparency

OpenRouter should be modeled as a routing gateway. The final model host, fallback provider, moderation layer, or endpoint privacy posture may differ from the user-visible gateway label.

Requirements:

- Store both `gateway_provider_id` and `routed_provider_id` when the provider API returns enough metadata to distinguish them.
- Show the active gateway and final routed provider/endpoint in request details when available. If unavailable, show `unknown`, not a guessed provider.
- Do not claim zero-data-retention, no-training, regional processing, or single-provider handling unless the selected account, key, provider preferences, and endpoint metadata prove it at request time.
- Treat fallback routing as a privacy-relevant event. A fallback from one model host to another must either follow an explicit user policy or stop for confirmation when the privacy class changes.
- Request logs may store routed provider metadata and request IDs, but not content-bearing generation retrieval payloads.
- Provider capability refresh must account for user provider preferences, disabled providers, privacy settings, guardrails, and endpoint availability where the gateway exposes those controls.


### Strict Provider Privacy Mode

Strict privacy mode sets provider constraints, not just UI disclosure. A strict-mode request must be constructed so the selected provider path satisfies the user's privacy constraints before any prompt, attachment, tool output, embedding, or conversation context leaves the device.

For OpenRouter, strict mode must:

- disable fallback routing where the OpenRouter request/account/provider interface supports a no-fallback or provider-restriction control;
- request provider data-collection denial where supported by the selected route/provider controls;
- avoid provider debug, metadata-expansion, stored-generation retrieval, or content-bearing diagnostics;
- verify that the selected model, account settings, provider preferences, and endpoint metadata still satisfy the strict policy immediately before payload transmission;
- stop before payload transmission if the provider cannot satisfy those constraints or if the app cannot prove that they were applied.

Strict mode failures are not retryable through a less private route. The UI may offer an explicit downgrade action, but the original strict request must remain unsent until the user chooses a weaker privacy class.

### Custom Endpoint Egress and SSRF Policy

OpenAI-compatible custom endpoints are an advanced feature and must be governed as an outbound network security boundary.

Default policy:

- Allow `https://` endpoints by default only after hostname validation, DNS resolution, and resolved-IP policy checks.
- Allow `http://` only for verified loopback endpoints such as `127.0.0.1`, `[::1]`, or `localhost` after resolution confirms loopback.
- Treat LAN/private-address endpoints as advanced and require explicit user approval plus a persistent warning that prompts may leave the device.
- Block cloud metadata addresses, link-local targets, multicast, broadcast, unspecified addresses, and private ranges unless the selected local-provider policy explicitly allows the exact target class.
- Resolve DNS immediately before connection, reject DNS rebinding where the resolved IP no longer matches the approved policy, and ensure the actual HTTP client connects to the validated target rather than re-resolving to a different address after validation.
- Disable automatic redirect following or revalidate every redirect target with the same scheme, hostname, resolved-IP, and TLS policy before continuing.
- Do not allow users or plugins to disable certificate validation for cloud endpoints.
- Do not send provider credentials or prompts until the endpoint has passed health checks that do not include user content.
- Log endpoint validation decisions as metadata only; never log prompt bodies or authorization headers.

### Prompt and Context Budget Policy

A desktop client can accidentally leak far more context than the user intended.

Required behavior:

1. Show selected context scope before send: current message only, selected conversation window, pinned memory, attachments, or artifact contents.
2. Default to the smallest useful context, not entire local history.
3. Never silently include unrelated conversations.
4. Provide a pre-send token estimate when possible.
5. Warn when large attachments or artifact contents are included.
6. Allow the user to inspect the exact local objects included in the request, even if not the provider-tokenized form.
7. Redact deleted or archived content from automatic context unless explicitly restored.


### OpenRouter Free-Tier Reality

OpenRouter is useful but not stable enough to be the only production path. Free-model limits and availability can change. The app must show quota-aware UX and support fallback options.

Required UI behaviors:

- Show when the selected model is a free variant.
- Show rate-limit waits without freezing the UI.
- When daily free quota appears exhausted, suggest paid model, another provider key, or local inference.
- Do not promise specific free models in static copy.

### Provider Abstraction

Define providers behind a trait/interface:

```rust
#[async_trait::async_trait]
trait ChatProvider {
    fn id(&self) -> ProviderId;
    async fn list_models(&self) -> Result<Vec<ModelCapability>, ProviderError>;
    async fn stream_chat(
        &self,
        request: ProviderChatRequest,
        sink: StreamSink,
        cancel: CancellationToken,
    ) -> Result<ProviderUsage, ProviderError>;
}
```

Initial providers:

| Provider | Priority | Notes |
|---|---|---|
| OpenRouter | Primary cloud provider | Broad model access, volatile free tier. |
| Ollama | Primary local fallback | Gives true local inference when installed. |
| OpenAI-compatible custom endpoint | Secondary | Allows advanced users to bring compatible gateways. |

### Local Provider Operational Checks

A local provider claim is valid only after runtime verification.

Required checks:

| Check | Requirement |
|---|---|
| Endpoint | Confirm endpoint is loopback or explicitly user-approved remote LAN address; validate scheme, hostname, resolved IP, redirects, and TLS policy before any prompt is sent. |
| Health | Probe API version/shape without sending user prompts. |
| Model inventory | List installed models and capabilities where possible. |
| Modality | Disable image/PDF/file controls unless the local model path supports them. |
| Context | Estimate and enforce model context limits locally. |
| Resource warning | Show RAM/VRAM/disk requirements when known. |
| Failure | If local model is unavailable, do not silently fall back to cloud. Ask or require a configured policy. |


---

## Performance and Footprint Optimization Strategies

### Binary and Bundle Budget

The target remains a small app, but the measurement must be explicit.

Define budgets per platform:

| Artifact | Budget | Notes |
|---|---:|---|
| Windows installed app directory | < 50 MB target | Exclude system WebView2 runtime if already installed; document assumptions. |
| macOS `.app` bundle | < 50 MB target | Measure uncompressed `.app`, not just `.dmg`. |
| Linux AppImage | Best effort | AppImage may exceed the target due to bundled dependencies. Track separately. |
| Frontend JS initial chunk | < 300 KB gzip target | Canvas/editor should be lazy-loaded. |
| Canvas/editor lazy chunk | < 2 MB gzip target | CodeMirror language imports must be controlled. |

Cargo release profile baseline:

```toml
[profile.release]
lto = true
opt-level = "z"
strip = true
panic = "abort"
codegen-units = 1
```

This profile may reduce binary size but can affect build time and debugging. CI should produce size reports for each target.

### Frontend Lazy Loading

The initial route should load only:

- core chat shell;
- conversation sidebar;
- model selector metadata;
- lightweight Markdown renderer shell.

Lazy-load:

- CodeMirror;
- language packages;
- diff engine;
- KaTeX;
- artifact preview tooling;
- heavy syntax highlighting.

### Performance Benchmarks

The implementation is not accepted until these benchmarks pass:

| Benchmark | Minimum Target |
|---|---|
| 50k-token streamed response | No sustained UI freeze > 100 ms after warmup |
| 1,000-message conversation | Sidebar and transcript remain scrollable |
| 100 code blocks | Syntax highlighting deferred; no progressive lockup |
| 50 KaTeX blocks | Math rendering deferred or batched |
| Long Markdown table | No unbounded layout thrashing |
| Low-end Windows machine | Basic streaming remains responsive |

Measure with browser performance APIs, Tauri logs, and automated fixtures.

---

## Annotated Comparison Tables

### Table 4: Local Storage and Persistence Solutions

| Solution | Performance & Search | Security & Isolation | Recommendation |
|---|---|---|---|
| SQLite via backend repository layer | Excellent with FTS5 and indexes | Stronger than frontend SQL because commands are constrained | Primary choice |
| Tauri SQL plugin exposed to frontend | Good | Risky if compromised frontend can issue arbitrary queries | Use only with tight capabilities, or avoid |
| Flat JSON files | Simple for export/debug | Weak for search and consistency | Reject for primary persistence |
| OPFS/WebView storage | Browser-like convenience | WebView/version lifecycle problems | Reject for primary persistence |
| Encrypted SQLite/SQLCipher-like approach | Good | Best privacy posture if implemented correctly | Recommended for privacy-forward release |

### Table 5: Editor Integrations

| Editor | Strengths | Risks | Recommendation |
|---|---|---|---|
| CodeMirror 6 | Modular, small, strong code editing model | Requires careful extension imports and state design | Primary choice |
| Monaco | VS Code-like editing | Larger footprint, weaker mobile/responsive fit | Reject for sub-50 MB target unless separately budgeted |
| Tiptap/ProseMirror | Excellent rich text documents | Not ideal for raw code artifacts | Optional for future document mode |

### Table 6: Streaming Implementations

| Approach | Strengths | Risks | Recommendation |
|---|---|---|---|
| Rust HTTP client + proper SSE parser + scoped Tauri events | Best control over credentials, cancellation, retries, and typed errors | Requires careful implementation | Primary choice |
| Frontend `fetch` | Simpler prototype | Exposes provider/network logic to webview and complicates secrets | Reject for production provider calls |
| WebSocket proxy | Useful for custom backends | More infrastructure | Future option only |

---

## Phased Implementation Roadmap

### Phase 0: Security Foundation and Product Truthfulness

- Define privacy modes: cloud, local, hybrid.
- Write onboarding copy that accurately states when prompts leave the device.
- Create Tauri window labels, capability files, and the source-controlled command inventory plus CI verifier.
- Disable production `app.withGlobalTauri`.
- Implement backend-only credential storage.
- Add command caller validation by window label.
- Decide plaintext vs encrypted SQLite.
- Add dependency pinning, license/SBOM baseline, native-binary review, and initial security audit.
- Add bundle-size reporting to CI.

Exit criteria:

- Frontend cannot read stored provider keys.
- Artifact preview has no Tauri IPC.
- Capability matrix and command-inventory verifier exist and are enforced in release configuration.
- Privacy copy is accurate.

### Phase 1: Core Chat and Persistence

- Implement provider abstraction.
- Implement OpenRouter provider with live model discovery.
- Implement Rust streaming with full SSE grammar parser.
- Implement stream IDs and scoped frontend events.
- Implement cancellation registry.
- Implement SQLite repository commands, metadata schema validation, and FTS query builder.
- Implement core conversation/message schema.
- Add WAL mode, controlled persistence cadence, and deletion/retention state model.

Exit criteria:

- One cloud model can stream safely.
- Stop Generation works and preserves partial output.
- No API key crosses into frontend memory after onboarding.
- Conversation history persists and reloads.

### Phase 2: Search, Quotas, and Resilience

- Implement FTS5 external-content table.
- Add insert/update/delete triggers.
- Add rebuild and integrity-check diagnostics.
- Implement quota-aware error handling.
- Implement retry budget and no-silent-retry-after-token rule.
- Add model capability UI gating.
- Add custom endpoint SSRF/redirect validation.
- Add local provider fallback stub.

Exit criteria:

- Search results remain consistent after inserts, updates, deletes, and migrations.
- Free-tier exhaustion has a graceful UX.
- Model modality controls are driven by runtime metadata.

### Phase 3: Canvas and Artifacts

- Add split-pane Canvas shell with static vs executable preview risk classes.
- Lazy-load CodeMirror and language extensions.
- Implement versioned artifact state.
- Implement AI patch review with revision-aware accept/reject.
- Implement sandboxed static HTML preview with `about:srcdoc` base hardening and iframe title; implement separate no-IPC executable preview surface before enabling generated JavaScript.
- Implement validated console bridge and keyboard-accessible escape/kill controls.
- Add malicious artifact test fixtures.

Exit criteria:

- AI-generated JS cannot call Tauri IPC.
- Console bridge cannot inject host UI HTML.
- Stale AI patches cannot overwrite newer user edits.

### Phase 4: Performance, Packaging, and Release Hardening

- Run full benchmark suite.
- Add size reports for Windows/macOS/Linux.
- Add dependency audit and license report.
- Add crash-safe persistence tests.
- Add backup/export/import tests.
- Add privacy-mode, provider-router, metadata-minimization, and deletion-semantics regression tests.
- Add accessibility release-gate tests.
- Add WAL checkpoint, single-instance, and backup/export consistency tests.
- Add exact adversarial fixtures for SSE errors, FTS query abuse, `srcdoc` escaping, WAL recovery, and capability drift.

Exit criteria:

- Size budgets are met or exceptions are documented.
- Benchmarks pass on target hardware.
- Security tests pass.
- Provider errors degrade gracefully.

### Release Evidence Bundle

Every release candidate must produce an evidence bundle:

```text
release-evidence/
  security-tests.txt
  streaming-tests.txt
  sse-error-fixtures.txt
  database-migration-tests.txt
  fts-integrity-tests.txt
  provider-drift-tests.txt
  provider-debug-retention-tests.txt
  provider-router-transparency-tests.txt
  metadata-minimization-tests.txt
  command-inventory-verifier.txt
  app-shell-csp-remote-asset-scan.txt
  deletion-retention-tests.txt
  fts-query-safety-tests.txt
  fts-query-abuse-fixtures.txt
  srcdoc-escaping-fixtures.txt
  wal-recovery-fixtures.txt
  capability-drift-fixtures.txt
  custom-endpoint-ssrf-tests.txt
  artifact-sandbox-tests.txt
  accessibility-tests.txt
  attachment-ingestion-tests.txt
  wal-checkpoint-single-instance-tests.txt
  performance-benchmarks/
  bundle-size-report.json
  dependency-audit.txt
  license-report.txt
  sbom.json
  update-signature-verification.txt
  platform-smoke-tests/
    windows.txt
    macos.txt
    linux.txt
```

A downstream agent may not mark the implementation complete without attaching or referencing this evidence.


---

## Required Test Matrix

### Security Tests

| Test | Expected Result |
|---|---|
| Main frontend attempts to call secret read command | Command unavailable or denied |
| Registered command missing from reviewed command inventory | CI/release build fails |
| Production build enables `app.withGlobalTauri` | Test fails |
| Ordinary UI window receives Stronghold record-read permission | Test fails |
| Artifact iframe attempts `window.__TAURI__` access | Undefined/inaccessible |
| Artifact sends malformed `postMessage` | Ignored |
| Artifact floods console bridge | Rate-limited |
| Compromised frontend attempts arbitrary SQL | No arbitrary SQL command exists |
| Wrong window label calls `set_provider_key` | Denied |
| API key appears in frontend logs | Test fails |
| Release build exposes devtools/debug commands | Test fails |
| Custom endpoint disables TLS validation | Test fails |
| Custom endpoint resolves to blocked private/link-local/metadata IP | Request denied before prompt/credential send |
| Custom endpoint redirect escapes approved policy | Redirect blocked |
| JavaScript supplies updater authorization header or private-feed token | Update check denied |
| Provider debug mode enabled in production | Test fails |
| Unsigned or wrong-channel update is offered | Update rejected |
| Crash report contains prompt fixture or key fixture | Test fails |
| Command appears in `generate_handler!` but not inventory | CI fails |
| Release capability grants command absent from inventory | CI fails |
| Frontend passes arbitrary absolute path to attachment-read command | Denied unless backed by valid Rust-issued token |
| Built app shell references CDN/remote script/style/font | Release scan fails |
| IPC payload exceeds command budget or violates schema | Rejected before provider/database/filesystem use |
| Metadata/log/support bundle contains prompt fixture, attachment fixture, or local path fixture | Test fails |
| Executable artifact infinite loop/message flood | Parent-owned kill/reload recovers host UI |
| Command inventory differs from `tauri_build::AppManifest::commands` | CI fails release |
| Capability file exists but is not selected for release and not marked dev-only | CI fails release |
| Strict privacy mode cannot disable fallback or request data-collection denial where required | Request stops before payload transmission |
| Stronghold default/read/enumerate/export permission reaches ordinary window | Test fails |
| Backend file-read command accepts raw JavaScript-returned path | Test fails |
| Artifact preview assigns raw generated HTML to `srcdoc` outside reviewed wrapper | Test fails |

### Streaming Tests

| Test | Expected Result |
|---|---|
| UTF-8 code point split across chunks | Correct output |
| SSE event split across chunks | Correct output |
| Multiple SSE events in one chunk | Correct output |
| SSE comment lines and blank-line dispatch | Comments ignored and event state remains valid |
| SSE multi-line `data:` fields | Data concatenated with newline separators before provider JSON parsing |
| SSE `id` and `retry` fields | Last-event-id and retry metadata handled without corrupting text |
| Provider sends error before body stream | Typed error |
| Cancel mid-stream | Partial output saved with `cancelled` status |
| 429 before first token | Bounded retry if budget remains |
| 429 after first token | No silent replay; user-visible continuation path |
| Provider sends usage in final streamed chunk | Usage stored without corrupting message text |
| Provider sends non-token error event mid-stream | Partial output preserved and typed error shown |
| Stream event from wrong stream ID/window | Ignored |
| Channel message from stale stream ID after a new stream starts | Ignored |
| Token payload delivered through global Tauri event | Test fails |

### Database Tests

| Test | Expected Result |
|---|---|
| Insert message | Message searchable |
| Update message content | Old terms removed, new terms searchable |
| Delete conversation | Deleted messages absent from search |
| Migration after existing messages | FTS rebuild indexes all rows |
| FTS integrity check | Passes |
| Export/import | Content preserved |
| Interrupted migration with WAL present | Recovery mode or successful rollback |
| WAL recovery fixture after power-loss simulation | Recovery mode, rollback, or repair path preserves durable user data |
| Long-lived reader blocks WAL checkpoint during backup/export | Operation blocks or fails safely with diagnostic |
| Second app instance starts during migration | First instance is focused/signaled or migration lock denies concurrent mutation |
| FTS index intentionally desynchronized | Integrity check fails and rebuild repairs |
| Delete attachment | Message remains valid and attachment unavailable |
| Malformed FTS query syntax | Validation error or literal fallback, no crash |
| FTS query with excessive operators/wildcards/length | Rejected by query budget |
| FTS query abuse fixture corpus | Validation/literal fallback succeeds without crash or unbounded scan |
| Delete conversation with attachments and FTS entries | Active rows, FTS rows, extracted text, and attachment records are removed or tombstoned consistently |
| Metadata schema receives raw provider JSON/content/path | Write rejected |

### Accessibility Tests

| Test | Expected Result |
|---|---|
| Keyboard-only core chat flow | User can select model, compose, send, stop, copy, and retry without pointer input |
| Focus visibility across chat, sidebar, settings, canvas, and preview | Visible focus is preserved in normal and high-contrast themes |
| Artifact iframe title and escape path | Screen-reader title exists and keyboard focus can return to host UI |
| Streaming/cancel/error status announcements | Status is programmatically determinable without stealing input focus |
| Destructive actions and exports | Labels, descriptions, and confirmations are programmatically associated |

### Performance Tests

| Test | Expected Result |
|---|---|
| Stream 50k tokens | UI remains responsive |
| Load 1,000-message conversation | Virtualization/collapse prevents freeze |
| Render many code blocks | Highlighting deferred or batched |
| Render KaTeX-heavy output | Math rendering does not block stream |
| Open artifact infinite loop | Kill/reload control recovers UI |
| Console bridge flood | Rate-limited without host UI freeze |
| Executable preview CPU loop | Host UI can recover through parent-owned kill/reload |
| Large rejected attachment | Clear error without reading entire file into memory |

---

## Implementation Anti-Patterns to Reject

Downstream agents must not implement any of the following:

- Passing `api_key` from Svelte to `stream_openrouter`.
- Storing provider keys in `localStorage`, `sessionStorage`, IndexedDB, plaintext JSON, or SQLite.
- Broadcasting token chunks globally instead of using per-invocation `tauri::ipc::Channel<StreamEvent>`.
- Treating raw byte chunks or isolated `data:` lines as complete SSE events.
- Ignoring SSE `event`, `id`, `retry`, comment, multi-line `data`, or blank-line dispatch semantics.
- Exposing arbitrary SQL execution to the frontend.
- Rendering AI-generated HTML directly into the main DOM.
- Using `sandbox="allow-scripts allow-same-origin"` for generated artifacts.
- Omitting `<base href="about:srcdoc">`, iframe titles, or keyboard escape paths from generated artifact previews.
- Silently retrying a generation after partial output has already appeared.
- Hardcoding OpenRouter free model names, quotas, context windows, or modalities.
- Claiming the app is “local-first” without local inference enabled.
- Importing all CodeMirror language packages by default.
- Writing every streamed token directly to SQLite.
- Allowing AI patches to apply against stale artifact revisions.
- Shipping release builds with devtools, broad dev capabilities, global `window.__TAURI__`, or debug commands enabled.
- Letting a JavaScript-returned absolute path authorize backend file reads.
- Granting Stronghold default/read/enumerate/export permissions to ordinary UI windows.
- Assigning generated preview HTML to `srcdoc` outside the reviewed escaping/wrapper function.
- Shipping release capabilities by directory presence rather than explicit `tauri.conf.json` selection.
- Treating strict privacy mode as disclosure-only instead of request-time provider constraints.
- Registering Tauri commands that are not in the reviewed command inventory.
- Granting ordinary UI windows Stronghold record-read permissions for secrets.
- Logging provider request bodies, authorization headers, prompt text, attachments, or artifact bodies.
- Silently falling back from local inference to cloud inference.
- Treating custom OpenAI-compatible endpoints as trusted without explicit user approval and SSRF/redirect validation.
- Following custom-endpoint redirects without revalidating scheme, host, resolved IP, and TLS policy.
- Enabling provider debug/metadata expansion or content-bearing generation retrieval in production without privacy review.
- Accepting archives, SVG/HTML, or oversized files through the attachment path without a threat-modeled parser.
- Running destructive SQLite migrations without backup/recovery strategy.
- Copying SQLite database files for backup/export while WAL is active without checkpoint/backup-API handling.
- Allowing concurrent app instances to race migrations, checkpoints, backups, destructive deletes, or update restarts.
- Installing unsigned updates, accepting update manifests from the wrong channel, or letting JavaScript supply updater auth headers.
- Implementing fake Tauri APIs such as a nonexistent compiled command inventory instead of a real project-owned verifier.
- Letting frontend strings directly select files for backend reads without Rust-owned user-selection proof.
- Treating `metadata_json` as a place to stash raw provider JSON, prompt snippets, completion text, extracted files, or absolute paths.
- Passing raw user search text directly into FTS5 `MATCH` without a query builder and budget.
- Loading CDN scripts, remote styles, remote fonts, analytics, or provider widgets into privileged app windows.
- Claiming “OpenRouter processed this” as if it identifies the final routed provider or retention posture.
- Running generated executable JavaScript in a same-renderer iframe without a verified host recovery path.
- Advertising deletion as remote/provider erasure without provider API proof and user-visible retention disclosure.

---

## Curated Resources and Reference Specifications

Use official or primary sources for normative behavior:

- Tauri v2 capabilities and permissions: https://v2.tauri.app/security/capabilities/ and https://v2.tauri.app/security/permissions/
- Tauri v2 IPC Isolation Pattern: https://v2.tauri.app/concept/inter-process-communication/isolation/
- Tauri v2 updater and signing: https://v2.tauri.app/plugin/updater/
- Tauri v2 configuration including `app.withGlobalTauri`: https://v2.tauri.app/reference/config/
- Tauri SQL plugin: https://v2.tauri.app/plugin/sql/
- Tauri Stronghold plugin: https://v2.tauri.app/plugin/stronghold/
- Tauri Rust/frontend command boundary: https://v2.tauri.app/develop/calling-rust/ and https://v2.tauri.app/develop/calling-frontend/
- OpenRouter quickstart and API overview: https://openrouter.ai/docs/quickstart
- OpenRouter model metadata endpoint: https://openrouter.ai/docs/api/api-reference/models/get-models
- OpenRouter rate limits: https://openrouter.ai/docs/api/reference/limits
- OpenRouter errors and debugging: https://openrouter.ai/docs/api/reference/errors-and-debugging
- Svelte 5 runes documentation: https://svelte.dev/docs/svelte/$state, https://svelte.dev/docs/svelte/$derived, https://svelte.dev/docs/svelte/$effect
- SQLite FTS5 official documentation: https://sqlite.org/fts5.html
- SQLite WAL official documentation: https://sqlite.org/wal.html
- SQLite backup API documentation: https://sqlite.org/backup.html
- CodeMirror 6 docs: https://codemirror.net/docs/guide/ and https://codemirror.net/docs/ref/
- MDN iframe documentation: https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/iframe
- MDN `srcdoc` documentation: https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/srcdoc
- MDN postMessage documentation: https://developer.mozilla.org/en-US/docs/Web/API/Window/postMessage
- MDN Server-Sent Events documentation: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events
- WHATWG Server-Sent Events parsing algorithm: https://html.spec.whatwg.org/multipage/server-sent-events.html
- OWASP SSRF Prevention Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html
- WCAG keyboard accessibility guidance: https://www.w3.org/WAI/WCAG21/Understanding/keyboard.html
- WCAG focus visible guidance: https://www.w3.org/WAI/WCAG21/Understanding/focus-visible.html
- WCAG status messages guidance: https://www.w3.org/WAI/WCAG21/Understanding/status-messages.html
- Candidate stream-aware Markdown package: https://github.com/humanspeak/svelte-markdown and https://www.npmjs.com/package/@humanspeak/svelte-markdown
- Candidate Svelte diff package: https://github.com/humanspeak/svelte-diff-match-patch

---

## Revision 5 Research Conformance Map

Revision 3 applied the research report's eight required hardening deltas. Revision 4 kept those corrections and added framework-version verification, file-intake, metadata, provider-transparency, and executable-preview hardening. Revision 5 converts the remaining high-risk advisory language into implementation blockers: deny-by-inventory Tauri commands, Channel-first streaming, explicitly selected release capabilities, provider privacy constraints, no frontend Stronghold read surface, opaque file-token intake, reviewed `srcdoc` assignment, and exact adversarial fixtures.

| Research Finding | Normative Architecture Change |
|---|---|
| Tauri command exposure | Added source-controlled command inventory, verifier, release capability cross-checks, and production-disabled `app.withGlobalTauri`. |
| Stronghold ambiguity | Restricted Stronghold to Rust-owned or frontend-inaccessible vault usage and rejected frontend store-record reads. |
| SSE underspecification | Upgraded streaming to full SSE grammar parsing before provider delta normalization. |
| Updater secret path | Required Rust-only updater credentials, private-feed headers, channel routing, rollback authorization, and JavaScript header rejection. |
| Provider debug/retention | Added production ban/default-off posture for provider debug, metadata expansion, and content-bearing retrieval surfaces. |
| Endpoint SSRF risk | Added custom endpoint scheme, DNS, resolved-IP, redirect, and TLS validation policy. |
| WAL lifecycle | Added WAL checkpoint policy, backup-safe copy rules, `busy_timeout`, and single-instance/migration coordination. |
| Artifact preview/accessibility | Added `about:srcdoc` base hardening, iframe titles, stricter message-channel requirements, keyboard escape paths, and accessibility release tests. |
| Command manifest drift risk | Clarified official Tauri `AppManifest::commands` usage and added a source-controlled command inventory plus project-owned CI verifier. |
| File intake path confusion | Made Rust-owned dialogs or opaque file-selection tokens mandatory before backend reads user files. |
| Executable preview denial of service | Split static iframe preview from executable generated-code preview and required no-IPC isolation plus parent-owned recovery. |
| FTS query safety | Added a bounded FTS5 query builder, literal fallback, parse-error handling, and adversarial query tests. |
| Metadata content leakage | Added allowlisted metadata schemas and tests preventing prompts, completions, files, paths, and raw provider JSON from hiding in metadata. |
| Provider-router opacity | Required gateway versus routed-provider metadata and prohibited unverified ZDR/retention/provider claims. |
| Deletion semantics | Added distinct local deletion, backup retention, export retention, provider retention, tombstone, and secure-erasure states. |
| App-shell CSP/remote assets | Blocked remote scripts/styles/assets in privileged windows and required release scans for CSP and remote references. |
| Deny-by-inventory command exposure | Required pinned Tauri v2 support for `tauri_build::AppManifest::commands`, build-script enumeration, and CI diffs against `generate_handler!`, release capabilities, and `security/command-inventory.toml`. |
| Channel-first streaming | Replaced global token events with per-invocation `tauri::ipc::Channel<StreamEvent>` plus backend-owned stream IDs and frontend stale-stream rejection. |
| Explicit release capabilities | Required `tauri.conf.json` to explicitly list release capabilities and CI to reject unselected non-dev capability files. |
| Strict provider privacy | Required request-time provider constraints for OpenRouter fallback and data-collection controls, with stop-before-send behavior on failure. |
| Stronghold frontend surface | Banned ordinary-window Stronghold default/read/enumerate/export permissions and limited credential access to domain commands. |
| File-token intake | Banned raw path read bridges and made raw JavaScript paths display hints only. |
| `srcdoc` assignment | Required a reviewed escaping/programmatic assignment wrapper that injects CSP/base/title and is tested by adversarial fixtures. |
| Exact adversarial fixtures | Added release evidence for SSE errors, FTS query abuse, `srcdoc` escaping, WAL recovery, and capability drift. |

---

## Final Hardened Architecture Statement

The production-grade version of this app is not merely “Tauri plus Svelte plus OpenRouter.” It is a **least-privilege desktop AI client** with local conversation ownership, explicit provider privacy boundaries, backend-only credentials, typed IPC, constrained persistence, runtime model discovery, Channel-first full-grammar streaming, governed endpoint egress, WAL-safe persistence, accessible UI flows, metadata minimization, Rust-owned file intake, and sandboxed artifact execution.

The stack remains viable:

- **Tauri v2** for the desktop shell and Rust security boundary.
- **Svelte 5** for a small, reactive UI.
- **SQLite/FTS5** for local history and fast search.
- **CodeMirror 6** for Canvas/code artifacts.
- **OpenRouter** as a convenient cloud provider, not the sole foundation.
- **Local inference providers** as necessary for any true local-first claim.

The decisive change is that every optimistic claim has been converted into a testable, security-preserving contract, and every framework-dependent assertion must correspond to the pinned dependency version, an actual API, a source-controlled verifier, or release evidence. Downstream implementation agents should treat this document as the source of truth and reject any shortcut that weakens the privacy boundary, IPC boundary, streaming contract, database consistency, or artifact sandbox.
