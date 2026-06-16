---
phase: 05-artifacts
plan: "01"
subsystem: database
tags:
  - tauri-v2
  - sqlite
  - ipc
  - artifact-sandbox
  - security
dependency_graph:
  requires:
    - phase: 04-privacy
      provides: opaque file-token intake and backend-owned command policy
  provides:
    - sanitized artifact srcdoc generation
    - persisted artifact rows in sqlite
    - post-completion artifact detection in chat streaming
    - minimal backend artifact IPC surface
  affects:
    - src-tauri/src/security
    - src-tauri/src/storage
    - src-tauri/src/ipc
    - src-tauri/permissions
tech_stack:
  added:
    - "none"
  patterns:
    - "backend-owned artifact detection after ChatEvent::Done"
    - "sanitize + CSP wrap before preview crosses IPC"
    - "append-only artifacts migration with typed store"
key_files:
  created:
    - src-tauri/src/storage/artifacts.rs
    - src-tauri/src/ipc/artifacts.rs
    - src-tauri/permissions/artifacts.toml
  modified:
    - src-tauri/src/security/artifact_sandbox.rs
    - src-tauri/src/security/command_policy.rs
    - src-tauri/src/storage/mod.rs
    - src-tauri/src/storage/migrations.rs
    - src-tauri/src/ipc/chat.rs
    - src-tauri/src/ipc/mod.rs
    - src-tauri/src/main.rs
    - src-tauri/capabilities/main.json
decisions:
  - "Stored artifact rows in SQLite with a simple discriminator + language column instead of a JSON blob so the schema stays queryable and the frontend never sees raw source."
  - "Kept artifact detection backend-owned and emitted ArtifactReady only after the stream completed and the artifact was persisted."
patterns-established:
  - "ArtifactStore owns persistence and preview regeneration."
  - "artifact_get re-sanitizes/re-wraps from backend source of truth."
requirements-completed:
  - ARTF-01
metrics:
  duration: session
  completed: 2026-06-15
---

# Phase 5: Artifacts Summary

**Backend artifact sandboxing, persistence, and chat-driven detection with typed artifact IPC.**

## Performance

- **Duration:** session
- **Completed:** 2026-06-15
- **Tasks:** 3
- **Files modified:** 8 relevant files plus new permission/IPC/store modules

## Accomplishments
- Implemented `security::artifact_sandbox` with sanitization for scripts, inline handlers, and `javascript:` URLs, plus strict CSP wrapping.
- Added the `artifacts` SQLite migration and a typed `ArtifactStore` that persists raw source backend-side and re-serves sanitized previews.
- Extended `chat_send` to resolve attachment tokens backend-side, detect artifacts only after `Done`, persist them, and emit `ArtifactReady`.
- Added backend artifact commands and capability/permission entries for `artifact_get` and `artifact_dismiss`.

## Task Commits

Not committed in this session; changes remain in the working tree.

## Files Created/Modified
- `src-tauri/src/security/artifact_sandbox.rs` - sanitizer + CSP wrapper for preview srcdoc.
- `src-tauri/src/storage/artifacts.rs` - typed artifact store, preview regeneration, and detection helper.
- `src-tauri/src/storage/migrations.rs` - append-only artifacts table migration.
- `src-tauri/src/ipc/chat.rs` - attachment token resolution, `ArtifactReady`, artifact persistence.
- `src-tauri/src/ipc/artifacts.rs` - `artifact_get` and `artifact_dismiss`.
- `src-tauri/src/main.rs` - register artifact store and commands.
- `src-tauri/permissions/artifacts.toml` - Tauri permissions for artifact commands.
- `src-tauri/capabilities/main.json` - main-window grants for artifact commands.

## Decisions Made
- Kept artifact preview generation entirely backend-owned.
- Stored artifact content type as a simple DB discriminator plus language column.
- Used a no-script `srcdoc` wrapper with a strict CSP and no `allow-same-origin` sandbox token.

## Deviations from Plan

None - plan executed as specified for the backend slice.

## Issues Encountered
- Tauri permission manifests needed an explicit `src-tauri/permissions/artifacts.toml` entry before the new capability grant would compile.

## Next Phase Readiness
- Backend artifact storage and IPC are in place for the frontend preview surface.
- Automated backend tests and cargo check pass.

---
*Phase: 05-artifacts*
*Completed: 2026-06-15*
