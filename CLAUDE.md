# Claude Code Instructions

This repository is a docs-led desktop AI client scaffold. Treat checked-in docs as the source of truth and the source tree as incomplete until runtime wiring and manifests are present.

## Top Rules

1. Read `AGENTS.md` before changing behavior, and read the nearest child `AGENTS.md` before editing a nested module.
2. Treat `docs/` and `.planning/` as the contract while the app is still scaffolded.
3. Keep backend-owned concerns backend-owned: secrets, file tokens, provider selection, storage, telemetry, and release evidence do not move into the renderer.
4. Keep the Rust/Tauri bootstrap thin. Push real behavior into named modules under `src-tauri/src/`.
5. Prefer the smallest correct change, update docs when behavior or boundaries change, and verify with the narrowest meaningful command set available.
