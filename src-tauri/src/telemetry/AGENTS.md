# telemetry/AGENTS.md

This subtree owns audit logging and release evidence.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/release-evidence.md`
3. `../../../docs/threat-model.md` — stub.
4. `../AGENTS.md`

## Purpose

Owns the audit log writer (`audit_log.rs`, append-only JSON-lines at the OS app-log directory) and release evidence capture (`release_evidence.rs`).

## Contracts & Invariants

- `AuditEntry` is constrained to exactly 4 fields (`timestamp`, `command`, `window`, `status`) by a regression test (`audit_entry_contains_only_metadata_fields` asserts `keys.len() == 4`) — never add content, arguments, or payload data to it.
- Audit log writes are best-effort: failures are intentionally ignored at the call site (`let _ = audit_log::write_audit_entry(...)`). A failed audit write must never block or fail the underlying command.
- `write_audit_entry` resolves and `create_dir_all`s the app-log directory on *every call*, not once at startup — it's a per-write filesystem check, not a cached handle.
- Telemetry must not leak secrets or private file contents; keep audit events structured and time-ordered; do not mix observability with behavior policy.

## Pitfalls

- Only `ipc::privacy` currently calls `audit_log::write_audit_entry` — `ipc::chat`, `ipc::history`, `ipc::artifacts`, and `ipc::files` don't, despite `history` being marked `sensitivity = "high"` in `command-inventory.toml`. Signals point to this being an intentional scope decision (privacy is the only module whose negative tests call out "never logs the raw key"), but no design doc confirms it — verify before assuming new sensitive commands should skip audit logging by default.

