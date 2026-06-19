# ipc/AGENTS.md

This subtree owns the commands the frontend can call.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/command-inventory.md`
3. `../../../docs/privacy-boundaries.md` — stub; see `../security/AGENTS.md` for what's actually enforced.
4. `../AGENTS.md`

## Purpose

One submodule per domain: `app_shell`, `chat`, `artifacts`, `history`, `privacy`, `files`, `inventory`. `providers.rs` is an unimplemented 1-line stub — provider-specific HTTP/SSE logic belongs in `providers::*`, never here.

## Contracts & Invariants

- Every error enum derives `thiserror::Error` + `serde::Serialize` with `#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]` → frontend always gets `{ code, message }`, never a raw panic.
- `ChatEvent` (the streaming union) uses a *different* tagging convention: `#[serde(tag = "type", rename_all = "PascalCase")]`. Don't copy the error-enum convention onto event types or vice versa.
- `chat_send` never accepts an `api_key` parameter (decision D-10) — credentials resolve server-side via `security::secrets::get_provider_key`. Only the type signature enforces this; there is no runtime check.
- Streaming terminal state (success, error, cancellation) is delivered *exclusively* through the `Channel<ChatEvent>`. `chat_send` itself always returns `Ok(())` once the request is accepted — never encode terminal state in a command's `Result`.
- Conversation titles are always backend-generated via `title_from_messages` (truncated to 60 Unicode scalar values), never accepted from IPC (decision D-03).
- Storage write failures inside the chat streaming task are non-fatal by design (mitigation T-03-13): logged via `eprintln!`, stream continues regardless. Don't "fix" this by propagating storage errors into the channel without checking with the team first — it's deliberate.
- `active_requests` in `AppState` is unconditionally cleaned up after every stream terminates (success/error/cancel) to prevent unbounded growth (mitigation T-02-04 / "Pitfall 5") — any new terminal path must clean it up too.
- IPC parameter names (Rust `snake_case`) must exactly match the frontend's `invoke(command, payload)` keys — there is no camelCase bridging.
- Each command should do one thing, validate input at the boundary, and return structured results.
- Do not place provider-specific logic in IPC handlers.

## Anti-patterns

- Adding a second way to signal stream completion/failure outside the `Channel<ChatEvent>`.
- Assuming `assert_main_window` alone protects a command — `command_policy::policy_check` additionally validates the command name against the `COMMANDS` allowlist; a command using only `assert_main_window` has no protection if an allowlist gate is later introduced as the sole check.

## Pitfalls

- `get_active_surface` previously released the shell lock between checking `hydrated` and reading SQLite, letting two concurrent callers both observe `hydrated == false` and race (fixed by holding the lock for the whole check-read-write sequence; commit `fc8dc6a`, CR-03). Any refactor of `app_shell.rs` hydration must keep the lock held end-to-end.
- `run_stream` races the HTTP connection attempt itself against the cancellation token via `tokio::select!` (`chat.rs:408-431`) — cancellation can fire before the first byte arrives, not just mid-stream.
- `AppShell.svelte` (a dead component that duplicated `SurfaceRail`, causing a double-rail DOM bug, CR-01) was removed from the layout in commit `db4f0a0`. `.planning/codebase/CONCERNS.md` claims it's still present — that doc is stale; verify in the actual `src/routes`/layout before assuming it needs removing again.

