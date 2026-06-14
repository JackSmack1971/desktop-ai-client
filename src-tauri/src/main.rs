// Prevent a console window from appearing on Windows in release builds.
// This attribute is stripped from debug/dev builds automatically by Rust.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod ipc;
mod providers;
mod security;
mod storage;
mod telemetry;

use app_state::AppState;
use storage::sqlite::{ShellPreferenceStore, SqlitePool};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        // Register the opener plugin so the app can open external URLs
        // from the backend (not exposed as a raw frontend command).
        .plugin(tauri_plugin_opener::init())
        // Bootstrap: open the SQLite database, run pending migrations, and
        // register SqlitePool and ShellPreferenceStore as managed state so
        // IPC command handlers can inject them via tauri::State.
        .setup(|app| {
            // Resolve the OS-provided per-app data directory and create it
            // on first launch so the SQLite file has a parent directory.
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;

            let db_path = dir.join("desktop-ai-client.db");

            // Open the database. run_migrations runs inside open() so the
            // shell_preferences table exists before any read or write.
            let pool = std::sync::Arc::new(SqlitePool::open(db_path)?);

            // Register SqlitePool so tauri::State<'_, SqlitePool> resolves.
            app.manage(pool.clone());

            // Register ShellPreferenceStore so tauri::State<'_, ShellPreferenceStore> resolves.
            app.manage(ShellPreferenceStore::new(pool));

            Ok(())
        })
        // Inject in-memory shell state. The backend-owned persistence layer
        // is handled by ShellPreferenceStore registered in the setup hook above.
        .manage(AppState::default())
        // Register backend-owned IPC commands. Only commands listed here
        // and in the reviewed command inventory are reachable from the
        // frontend. Keep `app.withGlobalTauri` false in tauri.conf.json
        // so frontend code must import the specific Tauri APIs it needs.
        .invoke_handler(tauri::generate_handler![
            ipc::app_shell::get_active_surface,
            ipc::app_shell::set_active_surface,
            ipc::chat::chat_send,
            ipc::chat::chat_cancel,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed to start");
}
