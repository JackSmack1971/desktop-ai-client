# Provider Routing

How the client chooses a provider, model, and bounded execution parameters
for a chat request — the Policy-Constrained Provider Runtime.

## Capability detection

`providers::capabilities::MODEL_ALLOWLIST` is the reviewed table of every
model the backend will route to. Each entry (`ModelSpec`) records:

- `id` — the provider-qualified model id (e.g. `anthropic/claude-sonnet-4-6`)
- `provider` — the owning `ProviderId` (only `OpenRouter` today)
- `is_default` — exactly one entry must set this; used when the renderer
  doesn't pin a model
- `supports_strict_privacy` — a reviewed claim about the provider's
  data-retention behavior for that model, not something verified at runtime.
  Treat changes to this field with the same care as
  `security/command-inventory.toml`: update it only after confirming the
  upstream provider's current policy.
- `max_completion_tokens_cap` / `default_max_completion_tokens`
- `min_temperature` / `max_temperature` / `default_temperature`

A model id not present in this table is never forwarded to a provider —
`providers::policy::resolve_execution_profile` rejects it with
`PolicyError::ModelNotAllowed`.

## Routing policy

`providers::policy::resolve_execution_profile(requested_model,
requested_max_completion_tokens, requested_temperature, privacy)` is the only
place that turns renderer-supplied request parameters into a bounded
`ExecutionProfile` plus the `RoutingDecision` that produced it:

1. Pick a candidate model: the caller's explicit pin, or the reviewed default
   when unpinned.
2. An explicitly-pinned model not in the allowlist is rejected outright
   (`ModelNotAllowed`) — never silently substituted.
3. `max_completion_tokens` and `temperature` are resolved against the chosen
   model's reviewed bounds (using the model's defaults when the caller didn't
   override them) and **rejected, not clamped**, when out of range
   (`MaxTokensOutOfRange` / `TemperatureOutOfRange`). Clamping would silently
   change what the caller asked for instead of surfacing the mismatch.

## Fallback behavior

Fallback only applies to the *privacy* dimension, and only when the caller
left the model unpinned:

- `privacy_mode = "strict"` with no `model` override: if the reviewed
  default model doesn't support strict privacy, the runtime falls back to
  `providers::capabilities::strict_privacy_fallback()` (the first
  allow-listed model that does) and sets
  `RoutingDecision.used_fallback = true`.
- `privacy_mode = "strict"` with an explicit `model` override that doesn't
  support it: **fails closed** (`PolicyError::PrivacyUnsatisfied`) instead of
  silently switching away from the model the caller explicitly named. A
  caller that cares enough to pin a model is treated as caring enough not to
  have it swapped out from under them.
- `privacy_mode = "standard"` (the default) never triggers a fallback.

## Provider drift handling

Once a `RoutingDecision` is resolved for a request, the backend does not
revisit it mid-stream: `providers::openrouter` and `providers::sse` use
exactly the model and parameters in the resolved `ExecutionProfile` for the
whole request. The model name surfaced in `ChatEvent::Done.model` is whatever
the provider actually reports for the completed response, which is a
separate signal from the *requested* `ExecutionProfile.model` — the two are
expected to match for OpenRouter today, but the protocol does not assume it.

`RoutingDecision.capability_hash` is a deterministic, non-cryptographic
fingerprint of `(provider, model, max_completion_tokens, temperature,
privacy)` — `providers::policy::capability_hash`, built on
`std::collections::hash_map::DefaultHasher` (fixed keys, so it is stable
across runs and processes, unlike a `HashMap`'s randomized `RandomState`).
It exists to make policy drift between "what was decided" and "what was
sent" detectable later (e.g. in a future audit-log integration), not to
gate any trust decision today.

## Message role validation

Every message crossing the IPC boundary — `history` and `new_message` — has
its `role` validated by `providers::policy::validate_message` before
`providers::routing::build_provider_messages` ever sees it. Only `"user"`
and `"assistant"` are accepted; anything else (most importantly `"system"`)
is rejected with `PolicyError::InvalidRole`. This is the boundary that stops
a hostile or buggy renderer from smuggling an unauthorized system-level
instruction into `history` to ride alongside the backend-owned system prompt
(`routing::DEFAULT_SYSTEM_PROMPT`, never accepted from IPC — D-12).

## Related

- Attachment intake limits: `security::attachment_budget`, documented in
  `docs/privacy-boundaries.md`.
- Full request flow: `docs/architecture.md` → "Policy-Constrained Provider
  Runtime".
