# Stack

## Summary

This repository is a scaffold for a desktop AI client built around a Tauri backend and a future frontend shell. The current source tree is mostly placeholder code, but the intended stack is clear from the module layout and Tauri config.

## Observed stack

- Rust backend crate under `src-tauri/src`
- Tauri desktop app shell configured in `src-tauri/tauri.conf.json`
- Frontend workspace scaffold under `src/` with no app framework committed yet
- Markdown docs as the primary source of product and boundary context
- Planned SQLite-backed persistence in `src-tauri/src/storage`
- Planned provider integration layer in `src-tauri/src/providers`
- Planned telemetry and audit layer in `src-tauri/src/telemetry`

## Not yet present

- No `Cargo.toml` was present in the workspace snapshot
- No `package.json` or frontend build config was present
- No real IPC handlers, provider clients, or storage implementations were present
- No test harness beyond empty test directories and fixtures

## Implication

The stack is currently a design scaffold, not an executable product. The repo is organized around the eventual runtime boundaries, but the implementation work still needs to fill in the application, frontend build, and backend crate plumbing.
