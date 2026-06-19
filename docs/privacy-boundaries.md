# Privacy Boundaries

This document will define what data the client may access, store, redact, or transmit.

## Focus areas

- secrets handling
- file content visibility
- command history retention
- telemetry redaction
- local storage scope

## Redaction

No redaction module exists yet. The earlier `security/redaction.rs` scaffold was
deleted (Phase 6 cleanup): it was an unconditional constant-return stub with no
call sites. Real redaction lands when something actually logs or persists
content that needs it.
