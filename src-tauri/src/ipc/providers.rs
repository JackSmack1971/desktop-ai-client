// TODO(PLAT-01): provider status/switching IPC commands are deferred — no
// roadmap phase is currently scheduled for PLAT-01 ("User can switch to
// local inference providers when available", see .planning/REQUIREMENTS.md).
// This module is intentionally unregistered: no #[tauri::command], no
// capability or permission file, no command_policy entry. Implement here
// once PLAT-01 is scheduled into a phase; until then this stays a no-op.
