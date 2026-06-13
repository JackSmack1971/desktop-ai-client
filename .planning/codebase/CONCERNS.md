# Concerns

## High-priority concerns

- The repository is almost entirely scaffold code, so docs currently describe more behavior than code enforces
- No Cargo or frontend package manifest was present, so the app is not yet buildable as-is
- Git was not initialized before this turn, which means the repo was not previously tracking its own planning state
- Security, provider routing, storage, and telemetry are split in docs, but not yet validated in implementation

## Product concerns

- The top-level docs describe a memory-first agent system, but the checked-in code does not yet realize that system
- The frontend shell is essentially empty, so UI behavior is still undefined
- The command inventory and privacy boundaries are documented, but there is no executable enforcement yet

## Delivery concerns

- The next implementation phase must create the missing manifests and wire the crate/app entrypoints
- Tests will need to be created from scratch and should start with boundary-level coverage
- Any future changes should keep the docs and code synchronized, or the repo will accumulate misleading architecture notes
