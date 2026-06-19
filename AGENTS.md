# Desktop AI Client

Monorepo for the desktop client shell, the Tauri backend, and supporting docs and tests.

## Intent Layer

Read the nearest `AGENTS.md` before editing code in a subdirectory.

- `src-tauri/AGENTS.md` - Rust/Tauri backend and nested backend modules

## Global Invariants

- Treat `docs/` and `.planning/` as the contract while the app is still scaffolded.
- Keep privacy boundaries explicit; redact sensitive data before logging or telemetry.
- Keep backend-owned concerns backend-owned: command policy, provider routing, storage, and telemetry stay out of the renderer.
- Prefer small, local AGENTS nodes when a subsystem has distinct ownership or invariants.

## Working Rules

- Prefer the smallest correct change, update docs when behavior or boundaries change, and verify with the narrowest meaningful command set.


==================================

Universal Agent Constitution
============================

    ENGINEERING CONSTITUTION


    1. Repository evidence outranks prior assumptions.
    2. Explicit requirements outrank inferred preferences.
    3. External verification outranks self-assessment.
    4. Public contracts must not change accidentally.
    5. Security boundaries must not be weakened for convenience.
    6. Tests must not be altered merely to conceal a defect.
    7. Existing unrelated user changes must be preserved.
    8. Scope expansion requires explicit justification.
    9. Uncertainty must be reported rather than disguised.
    10. Completion requires traceable evidence.
    11. Failed approaches must not be repeated without new evidence.
    12. Agents may recommend, but tools and artifacts establish facts.


