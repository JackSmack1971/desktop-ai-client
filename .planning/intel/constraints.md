# Constraints

## Architecture contract
source: `docs/architecture.md`
type: `protocol`

content:
- The app is a Tauri desktop client with a Svelte 5 renderer and a Rust backend, and renderer/backend communication stays on typed Tauri IPC commands only.
- Privacy-sensitive concerns stay backend-owned: provider credentials, file paths, SQLite storage, command policy, and telemetry never become renderer-owned state.
- `security::command_policy::policy_check` is the authority for IPC command access, and the command set must stay in sync with the reviewed inventory, capabilities, permissions, and handler registration.
- `chat_send` uses `history`, `new_message`, and `idempotency_key` to enforce the conversation transaction protocol; retries reuse the same turn identity instead of duplicating writes.
- The evidence-gated memory engine is Phase 1 shadow mode only; nothing in live chat or provider routing is allowed to depend on it yet.
- The policy-constrained provider runtime validates message roles, model allowlists, privacy mode, and attachment budgets before persistence or provider calls.

## Provider routing contract
source: `docs/provider-routing.md`
type: `api-contract`

content:
- `providers::capabilities::MODEL_ALLOWLIST` is the reviewed source of truth for routable models, defaults, privacy eligibility, and parameter bounds.
- `providers::policy::resolve_execution_profile` rejects unknown models and out-of-range token or temperature overrides instead of clamping them.
- Strict privacy only falls back when the model is unpinned; an explicitly pinned incompatible model fails closed.
- Routing decisions do not change mid-stream, and `RoutingDecision.capability_hash` is a deterministic drift-detection fingerprint rather than a trust boundary.
- Message-role validation only accepts `user` and `assistant` roles across `history` and `new_message`.

## Hardened architecture spec
source: `docs/Tauri_Svelte_AI_App_Architecture_Adversarial_Hardened_v5.md`
type: `nfr`

content:
- Production Tauri command exposure must be deny-by-inventory, with explicit release-capability selection and CI checks against the reviewed command inventory.
- Primary streaming must use backend-owned per-invocation channels, not broad global event-bus semantics.
- Strict provider privacy must become a request-time constraint, not just a UI disclosure, and the request must stop before payload transmission if the provider cannot satisfy it.
- Ordinary frontend windows must not receive Stronghold read or export surfaces; secret access stays Rust-owned or backend-inaccessible.
- File intake must be Rust-owned or tokenized, with opaque file tokens, metadata-only checks, and bounded attachment handling before any content is read.
- Artifact previews, SSE grammar handling, metadata minimization, and provider-router transparency all have explicit hardening requirements that must be preserved.
