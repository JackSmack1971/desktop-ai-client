---
name: testing
description: Test-driven development, coverage rules, and verification expectations.
paths:
  - "src/**/*.{test,spec}.{ts,js}"
  - "src/**/*.{test,spec}.{tsx,jsx}"
  - "tests/**/*.{test,spec}.{ts,js,tsx,jsx,rs}"
  - "e2e/**/*.{test,spec}.{ts,js,tsx,jsx}"
  - "src/lib/components/**/*.{svelte,ts}"
  - "src/routes/**/+*.{svelte,ts,js}"
  - "src-tauri/src/**/*.rs"
  - "src-tauri/tests/**/*.rs"
  - "playwright.config.{ts,js}"
  - "vitest.config.{ts,js}"
  - "docs/release-evidence.md"
  - ".planning/codebase/TESTING.md"
---

# Testing Rules

## Baseline

- Behavior-changing work needs the smallest meaningful regression test or explicit verification note.
- Start with the boundary closest to the change: backend commands, provider routing, storage recovery, privacy redaction, or preview sandboxing.
- Keep security and privacy regressions hostile by default. Test the failure mode, not just the happy path.
- When a change cannot be tested yet, say exactly what is missing and why.
- Do not treat compilation alone as sufficient proof when boundaries or release evidence changed.

## Frontend, Route, and Component Coverage

- Add at least 1 success-path test for each changed `+page.server.ts`, `+layout.server.ts`, `+server.ts`, or server action with branching logic.
- Add at least 1 expected-failure test for each introduced `error(401, ...)`, `error(403, ...)`, or `error(404, ...)` branch.
- Use Vitest for module-level server logic, validators, stores, and pure `$lib/**` functions.
- Use Playwright for browser navigation, progressive-enhancement forms, authentication flows, and hydrated UI behavior.
- Keep test fixtures scoped to `tests/**`, `e2e/**`, or a co-located `__fixtures__` directory with clear route or module ownership.
- Run `pnpm run check` after edits to route filenames, route params, `src/app.d.ts`, `load` functions, actions, or generated `$types` consumers.
- Run the narrowest relevant Vitest command after unit-level changes, then record the exact command and result in the handoff or PR notes.
- Run the narrowest relevant Playwright command after e2e-level changes, then record the exact command and result in the handoff or PR notes.
- Run `pnpm run build` after changes to SvelteKit config, adapters, server-bound module boundaries, or environment usage.
- Run `npx svelte-check --tsconfig ./tsconfig.json` after changes touching matched `.svelte` paths that alter props, snippets, callback props, local markup branches, or route typing.

## Frontend Component Verification

- Run the project Svelte/TypeScript check script after editing any `.svelte` file that changes props, snippets, event callbacks, or local markup branches.
- Add or update at least 1 component test when a change modifies visible state transitions, callback payloads, or conditional rendering.

## Backend and Tauri Coverage

- Add at least 1 test for every new `#[tauri::command]` covering the success payload and the error payload.
- Validate TypeScript IPC wrapper payload keys against Rust command parameter names with a dedicated test or type-level assertion.
- Run `cargo test --manifest-path src-tauri/Cargo.toml` for Rust command, state, plugin, and capability-adjacent changes.
- Run `pnpm test` or the package-manager equivalent for frontend wrappers that call `@tauri-apps/api/*`.
- Execute `pnpm tauri build` or the package-manager equivalent after capability, permission, CSP, bundle, or updater changes.
- Store deterministic fixtures for window labels, command payloads, and channel event enums under `tests/fixtures` or `src/**/__fixtures__`.

## Release Evidence

- Keep release evidence minimal, specific, and reviewable.
- Record any explicit verification note with the exact command, outcome, and unresolved gap if the change could not be tested directly.
