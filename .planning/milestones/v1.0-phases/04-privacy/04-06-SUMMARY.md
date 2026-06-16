---
phase: 04-privacy
plan: 06
status: completed
tags: [rust, tauri, capabilities, permissions]
---

# Phase 04 Plan 06 Summary

Wired the new command surface into Tauri:

- Registered the five Phase 4 commands in `generate_handler!`
- Added the dialog plugin initialization
- Added `privacy.toml` and `files.toml`
- Granted the new `allow-*` identifiers in `capabilities/main.json`

Verification:

- `cargo build --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`

