---
status: partial
phase: 01-app-shell
source: [01-VERIFICATION.md]
started: 2026-06-13T20:55:00Z
updated: 2026-06-13T20:55:00Z
---

## Current Test

[awaiting human testing]

## Tests

### 1. App launch and surface display

expected: Run `npm run dev` and confirm the shell opens into WorkspaceShell with four labeled surface tabs and no IPC errors in the DevTools console. Requires Tauri native runtime with `cargo` available.
result: [pending]

### 2. Keyboard-only navigation

expected: Roving tabindex, Arrow/Home/End/Enter/Space behavior, and visible focus ring all function correctly in the running WebView. Tab moves between the rail and main content; arrow keys move within the rail; Enter/Space activates.
result: [pending]

### 3. Screen reader announcements

expected: The `aria-live="polite"` status region fires on surface switch. Only one `role="application"` appears in the accessibility tree (WorkspaceShell only).
result: [pending]

### 4. Restart persistence

expected: Switch to a non-Chat surface, quit the app, relaunch — the shell restores to the chosen surface. Requires full Tauri process lifecycle (not dev server reload).
result: [pending]

### 5. Rust test suite

expected: `cargo test --workspace --all-targets` exits 0. All five integration tests in `src-tauri/tests/app_shell.rs` pass. This is a prerequisite for items 1–4 (the build must succeed before the app can launch).
result: [pending]

## Summary

total: 5
passed: 0
issues: 0
pending: 5
skipped: 0
blocked: 0

## Gaps
