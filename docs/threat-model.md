# Threat Model

The desktop client threat model.

## Focus areas

- provider routing abuse
- secret exposure
- hostile renderer behavior
- unsafe command execution
- file access boundary violations

## Mitigated: renderer role injection (hostile renderer behavior)

**Threat:** a hostile or compromised renderer calls `chat_send` with a
`history` entry whose `role` is `"system"` (or any other value), attempting
to inject an unauthorized system-level instruction that rides alongside the
backend-owned system prompt (`providers::routing::DEFAULT_SYSTEM_PROMPT`,
never accepted from IPC per D-12) and gets undue trust from the model.

**Mitigation:** `providers::policy::validate_message` rejects any role
outside `"user"` / `"assistant"` for every message in `history` and
`new_message`, before persistence or provider routing. See
`docs/provider-routing.md` → "Message role validation".

## Mitigated: unconstrained renderer model/parameter override (provider routing abuse)

**Threat:** a renderer passes an arbitrary `model` string, or
`max_completion_tokens`/`temperature` outside any sane range, directly to the
provider — enabling cost abuse, requests to an unreviewed model, or
provider-side errors from out-of-range parameters.

**Mitigation:** `providers::policy::resolve_execution_profile` resolves
every request against the reviewed allowlist in `providers::capabilities`
and rejects (does not clamp) out-of-bounds token/temperature overrides. See
`docs/provider-routing.md`.

## Mitigated: unsatisfiable privacy requirement silently downgraded

**Threat:** a caller requests strict (e.g. zero-data-retention) handling,
but the resolved model doesn't support it; if the backend silently used a
non-compliant model anyway, the caller would believe a privacy guarantee was
honored when it wasn't.

**Mitigation:** `PrivacyMode::Strict` fails closed when an explicitly-pinned
model can't satisfy it (`PolicyError::PrivacyUnsatisfied`); fallback to a
compliant model only happens when the caller left the model unpinned. See
`docs/privacy-boundaries.md` → "Privacy mode".

## Mitigated: unbounded attachment ingestion (file access boundary violations)

**Threat:** an attached file of arbitrary size or type causes an unbounded
in-memory read and an unbounded outbound payload to a third-party provider;
a binary file gets lossily decoded into garbage text and shipped anyway.

**Mitigation:** `security::attachment_budget` checks file count, per-file
size, total size, an estimated-token ceiling, and a text-like MIME allowlist
against metadata alone, before any content is read. See
`docs/privacy-boundaries.md` → "Attachment intake".

## Mitigated: file token map unbounded growth (secret/path-adjacent exposure surface)

**Threat:** `security::file_tokens` mints a token per attached file but
never revoked one, so a long session's token-to-path map grows unbounded in
memory for the life of the process.

**Mitigation:** `ipc::chat::resolve_attachments` revokes a token immediately
after its content is successfully read. A token rejected by the attachment
budget is left valid so the caller can retry.

## Deferred by design: memory engine retrieval is not yet live

**Threat:** once a future phase wires `storage::memory::bounded_retrieve`
into `chat_send`, a promoted candidate whose `summary` was derived from
attacker-influenced conversation content could be replayed back into a
later prompt as trusted context — a stored-prompt-injection path distinct
from the renderer-role-injection threat above.

**Current mitigation:** Phase 1 deliberately keeps `storage::memory`
unreachable from `ipc::chat`/`providers::routing` (see
`docs/architecture.md` → "Evidence-Gated Memory Engine"). No live prompt
can be influenced by a memory candidate yet, so this threat has no exploit
surface today. Revisit this entry — with an explicit mitigation (e.g.
re-validating a promoted candidate's provenance before injection, or
treating retrieved memory text the same as untrusted renderer input) —
before any phase wires retrieval into a live prompt.

## Open / not yet addressed

- `providers::capabilities::ModelSpec.supports_strict_privacy` is a reviewed
  config claim, not something verified against the live provider at
  request time — see `docs/provider-routing.md`.
- `providers::policy::PolicyReceipt` is logged via `eprintln!` only; it is
  not yet persisted through `telemetry::audit_log` (see
  `docs/privacy-boundaries.md` → "Audit-safe receipts").
- `RoutingDecision.capability_hash` is a non-cryptographic fingerprint for
  drift detection, not a security boundary by itself.
