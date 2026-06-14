# Roadmap: Desktop AI Client

## Phase 1: App Shell

**Goal:** Get the desktop app booting into a usable workspace shell with clear navigation boundaries.
**Mode:** mvp
**Requirements:** SHELL-01, SHELL-02
**Success Criteria**:

1. The app launches successfully from the desktop shell.
2. The user can reach the main workspace and switch between the major surfaces.
3. The shell is organized so backend and frontend boundaries remain explicit.

## Phase 2: Routing

**Goal:** Route user prompts through deterministic provider selection and robust streaming transport.
**Mode:** mvp
**Requirements:** ROUTE-01, ROUTE-02
**Success Criteria**:

1. A prompt can be routed without frontend ownership of provider secrets.
2. Streaming output arrives in order and preserves partial output.
3. Cancellation and typed error handling work without corrupting the active stream.

## Phase 3: History

**Goal:** Persist and search local conversation history with recoverable storage behavior.
**Mode:** mvp
**Requirements:** HIST-01, HIST-02, HIST-03
**Plans:** 3/4 plans executed
**Success Criteria**:

1. Conversations persist across app restarts.
2. Prior messages are searchable through local history.
3. Retention and deletion behavior are explicit and testable.

Plans:

- [x] 03-01-PLAN.md — SQLite schema migrations (0002, 0003) + typed domain stores (ConversationStore, MessageStore, FtsStore, RetentionStore)
- [x] 03-02-PLAN.md — IPC command surface (history_list, history_get, history_delete, history_search) + main.rs registration + capabilities
- [x] 03-03-PLAN.md — chat_send storage wiring (conversation persistence, title generation, Done/Cancel terminal writes)
- [ ] 03-04-PLAN.md — Frontend History surface (historyStore, HistorySurface, SearchBar, ConversationList, ConversationRow)

## Phase 4: Privacy

**Goal:** Enforce the privacy boundary for secrets, file access, and telemetry.
**Mode:** mvp
**Requirements:** SEC-01, SEC-02, SEC-03
**Success Criteria**:

1. Ordinary frontend windows cannot read backend-owned secrets.
2. File access rejects raw frontend path authority.
3. Logs and telemetry redact sensitive content before persistence or transmission.

## Phase 5: Artifacts

**Goal:** Provide sandboxed artifact previews that remain safe and usable under hostile content.
**Mode:** mvp
**Requirements:** ARTF-01, ARTF-02, ARTF-03
**Success Criteria**:

1. Generated artifacts render inside a constrained preview surface.
2. A runaway artifact can be stopped or reloaded without freezing the host UI.
3. Keyboard and screen-reader paths remain usable for preview workflows.

## Phase 6: Release

**Goal:** Make the project release-ready with reviewed command exposure and adversarial evidence.
**Mode:** mvp
**Requirements:** REL-01, REL-02
**Success Criteria**:

1. Command exposure is explicitly inventoried and cross-checked before release.
2. The release gate includes the expected security, routing, storage, and fixture evidence.
3. A build alone is not considered complete unless the verification evidence is present.
