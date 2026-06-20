# Privacy Boundaries

What data the client may access, store, redact, or transmit.

## Focus areas

- secrets handling
- file content visibility
- command history retention
- telemetry redaction
- local storage scope
- privacy mode (Policy-Constrained Provider Runtime)

## Redaction

No redaction module exists yet. The earlier `security/redaction.rs` scaffold was
deleted (Phase 6 cleanup): it was an unconditional constant-return stub with no
call sites. Real redaction lands when something actually logs or persists
content that needs it.

## Privacy mode

`chat_send` accepts an optional `privacy_mode` ("standard" or "strict"),
resolved by `providers::policy::resolve_execution_profile` against the
reviewed model allowlist in `providers::capabilities`:

- **Standard** (default) — any allow-listed model may be used.
- **Strict** — the resolved model must be reviewed as eligible for
  zero-data-retention-style handling (`ModelSpec.supports_strict_privacy`).
  If it isn't, and the caller didn't pin a specific model, the runtime falls
  back to a model that does support it. If the caller *did* pin an
  incompatible model explicitly, the request **fails closed**
  (`PolicyError::PrivacyUnsatisfied`) — it is never silently downgraded to
  standard handling. See `docs/provider-routing.md` for the full resolution
  order.

An unrecognized `privacy_mode` value is rejected
(`PolicyError::InvalidPrivacyMode`), not defaulted — a typo in this
parameter must surface as an error, not silently fall back to the weaker
tier.

## Attachment intake

`security::attachment_budget` enforces, on metadata only — before any
attachment content is read:

- file count (`AttachmentBudget::max_files`)
- per-file byte size (`max_bytes_per_file`)
- total byte size across the request (`max_total_bytes`)
- an estimated-token ceiling (`max_estimated_tokens`, bytes / 4)
- a text-like MIME allowlist (`text/*`, `*/json`, `*/xml`) — binary files are
  rejected outright rather than lossily decoded into garbage text and shipped
  to a provider

`security::file_tokens` issues opaque, single-use tokens for file intake; the
raw path never crosses the IPC boundary. `ipc::chat::resolve_attachments`
revokes a token immediately after its content is successfully read, so the
token map (`AppState.file_tokens`) cannot grow unbounded across a session. A
token rejected by the attachment budget is left valid (the caller can retry
with a smaller attachment set) — only a *successful* read consumes it.

## Message role boundary

Backend-owned system prompt content (`providers::routing::DEFAULT_SYSTEM_PROMPT`)
is never accepted from IPC. `providers::policy::validate_message` enforces
that every renderer-supplied message role is exactly `"user"` or
`"assistant"` before it reaches provider routing, closing the path where a
renderer-asserted `"system"` role could otherwise be smuggled into `history`
alongside the real system prompt.

## Memory engine storage (Phase 1, shadow mode)

`storage::memory` (see `docs/architecture.md`'s "Evidence-Gated Memory
Engine" section) persists derived summaries of conversation content
(`memory_run_traces.task_summary`, `memory_candidates.summary`) in new
SQLite tables, separate from `messages`. Retention compliance carries over
unchanged: every memory row chains back to a `conversation_id` via `ON
DELETE CASCADE` (`memory_run_traces` → `conversations`, `memory_candidates`
→ `memory_run_traces`, `memory_decisions` → `memory_candidates`), so
`RetentionStore::delete_conversation`'s hard delete already removes a
conversation's memory rows with no separate purge step needed. This module
is not reachable from any IPC command — there is no new renderer-facing
surface to review here, only a backend-internal pipeline exercised by tests
and `telemetry::memory_replay`.

## Audit-safe receipts

`providers::policy::PolicyReceipt` (resolved model id, fallback flag, privacy
mode, capability hash) is constructed for every `chat_send` call and is safe
to log or display — it never contains a secret, a raw file path, or prompt
content. It is currently logged via `eprintln!` only; persisting it through
`telemetry::audit_log` is a natural follow-up once that module's audit
schema is extended beyond `{command, window, status}`.
