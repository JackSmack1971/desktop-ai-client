# src-tauri

Tauri crate boundary for the desktop backend, build/runtime config, and backend persistence setup.

## Read First

Before editing code here, read:
1. `../AGENTS.md`
2. `../docs/architecture.md` — **caution:** describes an unrelated Planner/Executor/Memory/Judge agent system, not this Tauri/Svelte/Rust client. Use `../.planning/codebase/ARCHITECTURE.md` instead for current-state claims.
3. `../docs/privacy-boundaries.md` — stub (focus-area list); documents only that `security/redaction.rs` was deleted as an unused stub.
4. `../docs/threat-model.md` — stub (header + 5-item focus-area list, no actual analysis).
5. `src/AGENTS.md`

## Purpose

Owns the Rust backend crate, Tauri configuration, and migration assets. Does not own frontend UI code or top-level docs.

## Entry Points

- `src/main.rs` - Backend bootstrap entrypoint; registers every `#[tauri::command]` via `tauri::generate_handler![...]` (lines 63-79)
- `src/app_state.rs` - Shared runtime state (`AppState`)
- `tauri.conf.json` - Tauri app configuration
- `migrations/` - Backend migration assets
- `src/bin/verify-command-inventory.rs` - cross-checks command registration consistency

## Contracts & Invariants

- **Triple command registration**: every frontend-callable command needs (1) `tauri::generate_handler!` in `main.rs`, (2) a capability entry in `capabilities/main.json`, and (3) a row in `security/command-inventory.toml`. `verify-command-inventory` enforces this; adding only one or two is a silent gap.
- **Lock ordering**: the `shell` mutex in `AppState` must be acquired before the SQLite connection mutex inside `ShellPreferenceStore`. This was a documented race-condition fix (CR-03, commit `fc8dc6a`) — reordering reintroduces it.
- **`tauri::State<'_, T>` is not `'static`** and cannot be moved into `tokio::spawn`. Re-acquire the store via `app_handle.state::<T>()` inside the spawned task body instead (see `ipc::chat.rs` "Pitfall 1", e.g. lines 179, 189, 267, 276, 311, 320, 330, 342).
- Secrets never cross an `.await` point and never reach a log macro, error format string, or IPC response field. Use `secrecy::SecretString`.
- Keep Tauri bootstrap thin; push behavior into named backend modules.
- Do not duplicate frontend concerns here.

## Anti-patterns

- Registering a command in only `main.rs` without the matching `capabilities/main.json` entry and `command-inventory.toml` row.
- Capturing a `State` guard inside a `tokio::spawn` closure instead of re-acquiring it from the `app_handle`.
- Treating a local `assert_main_window` check as equivalent to `security::command_policy::policy_check` — the latter also validates the command name against an allowlist; the former only checks the window label. See `src/security/AGENTS.md`.

## Pitfalls

- `security::command_policy::COMMANDS` (the allowlist `policy_check` consults) omits `get_active_surface` and `set_active_surface`, even though both are registered in `main.rs` and listed in `command-inventory.toml`. They are protected only by `ipc::app_shell`'s local `assert_main_window`, never by the centralized policy — don't assume every registered command is allowlist-covered.
- Window-label ("main"-only) enforcement is implemented three different, currently-active ways across IPC modules: a duplicated private `assert_main_window` helper (`app_shell`, `chat`, `history`), the centralized `command_policy::policy_check` (`files`, `privacy`), and both called redundantly in sequence (`artifacts`). There is no single source of truth — check which pattern a module already uses before adding a new command to it.

## Related Context

- Shared backend modules: `src/AGENTS.md`

