/// Frontend-facing command surface.
///
/// Each submodule owns one domain of IPC commands. Commands must:
/// - Validate caller window label (backend-side enforcement supplements capabilities)
/// - Validate input at the boundary before handing off to business logic
/// - Return structured, typed results
/// - Never expose secrets, raw paths, or arbitrary SQL to the frontend
///
/// The full list of registered commands must remain in sync with:
/// - `tauri::generate_handler![...]` in main.rs
/// - `src-tauri/capabilities/*.json` capability grants
/// - `security/command-inventory.toml` (reviewed inventory)
pub mod app_shell;
pub mod chat;
pub mod files;
pub mod history;
pub mod inventory;
pub mod privacy;
pub mod providers;
