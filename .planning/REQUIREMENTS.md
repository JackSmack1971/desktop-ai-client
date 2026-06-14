# Requirements: Desktop AI Client

**Defined:** 2026-06-13
**Core Value:** Keep local history, files, and agent state private while safely routing AI inference, streaming, and artifacts through explicit backend boundaries.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Shell

- [x] **SHELL-01**: User can launch the desktop app and reach the main workspace
- [x] **SHELL-02**: User can navigate between chat, history, settings, and artifact surfaces

### Routing

- [ ] **ROUTE-01**: User can send a prompt through deterministic provider selection
- [ ] **ROUTE-02**: User can receive streamed assistant output in order, including partial output and cancellation

### History

- [ ] **HIST-01**: User can save conversations locally
- [ ] **HIST-02**: User can search prior conversations and messages
- [ ] **HIST-03**: User can delete or retain history according to configured retention rules

### Security

- [ ] **SEC-01**: Secrets stay backend-owned and are not exposed to ordinary frontend windows
- [ ] **SEC-02**: File access uses opaque tokens or Rust-owned selection instead of raw frontend paths
- [ ] **SEC-03**: Sensitive data is redacted before logs or telemetry

### Artifacts

- [ ] **ARTF-01**: User can preview generated artifacts in a sandboxed host-controlled surface
- [ ] **ARTF-02**: User can stop or reload a runaway preview without losing the host UI
- [ ] **ARTF-03**: Artifact previews remain keyboard accessible and expose usable status feedback

### Release

- [ ] **REL-01**: Builds include a reviewed command inventory and explicit release capability selection
- [ ] **REL-02**: Release evidence covers security, routing, storage, and adversarial fixtures

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Platform Extensions

- **PLAT-01**: User can switch to local inference providers when available
- **PLAT-02**: User can manage multi-agent workflows from the desktop client

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Raw SQL execution from the frontend | Persistence must stay behind typed backend commands |
| Unrestricted remote assets in privileged windows | Production attack surface must stay narrow |
| Frontend secret reads from Stronghold or equivalent vaults | Secret storage stays backend-owned |
| Raw JavaScript-returned path reads | File access must use Rust-owned selection or tokens |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SHELL-01 | Phase 1 | Complete |
| SHELL-02 | Phase 1 | Complete |
| ROUTE-01 | Phase 2 | Pending |
| ROUTE-02 | Phase 2 | Pending |
| HIST-01 | Phase 3 | Pending |
| HIST-02 | Phase 3 | Pending |
| HIST-03 | Phase 3 | Pending |
| SEC-01 | Phase 4 | Pending |
| SEC-02 | Phase 4 | Pending |
| SEC-03 | Phase 4 | Pending |
| ARTF-01 | Phase 5 | Pending |
| ARTF-02 | Phase 5 | Pending |
| ARTF-03 | Phase 5 | Pending |
| REL-01 | Phase 6 | Pending |
| REL-02 | Phase 6 | Pending |

**Coverage:**

- v1 requirements: 15 total
- Mapped to phases: 15
- Unmapped: 0 ✓

---
*Requirements defined: 2026-06-13*
*Last updated: 2026-06-13 after initial definition*
