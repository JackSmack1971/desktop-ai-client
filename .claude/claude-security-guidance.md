# Claude Security Guidance

Use this guidance when reviewing code, prompts, hooks, workflows, or release evidence for `desktop-ai-client`.

## High-Risk Surfaces

- provider credentials and secret stores
- file selection and attachment intake
- raw prompt logs and retained conversation history
- telemetry, audit logs, and release evidence
- renderer-to-backend IPC commands
- preview surfaces for generated artifacts
- destructive storage or migration actions

## Review Priorities

1. Prefer backend-owned secrets and typed IPC over renderer access.
2. Redact sensitive content before logs, telemetry, or evidence capture.
3. Treat raw path authority as hostile input unless explicitly tokenized.
4. Keep command execution narrow, explicit, and reviewable.
5. Keep preview windows sandboxed and recoverable.

