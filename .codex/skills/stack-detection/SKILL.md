---
name: stack-detection
description: Classify the desktop client stack before architectural or client-layer changes.
disable-model-invocation: false
user-invocable: true
---

# Stack Detection

Inspect:

- `package.json` or other frontend build manifests
- `src-tauri/Cargo.toml` and Rust entrypoints
- `src-tauri/tauri.conf.json`
- `docs/` and `.planning/`
- the actual source tree under `src/` and `src-tauri/src/`

Classify:

- `docs-first-scaffold`: architecture is described in docs but not yet enforced by runnable app code
- `tauri-desktop-app`: backend/runtime wiring is present and the app is buildable
- `mixed-or-partial`: the project has some runtime wiring but still carries important scaffold gaps

Output:

- Chosen classification
- Evidence files
- Missing build or runtime manifests
- Follow-on skill suggestions:
  - `privacy-boundary-review`
  - `provider-routing-review`
  - `storage-recovery-review`
  - `release-evidence-review`

