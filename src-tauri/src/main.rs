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
use storage::artifacts::ArtifactStore;
use storage::fts::FtsStore;
use storage::retention::RetentionStore;
use storage::sqlite::{ConversationStore, MessageStore, ShellPreferenceStore, SqlitePool};
use storage::turns::TurnStore;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        // Register the opener plugin so the app can open external URLs
        // from the backend (not exposed as a raw frontend command).
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            app.manage(ShellPreferenceStore::new(pool.clone()));

            // Conversation Transaction Protocol: recover any turn attempt
            // left `in_progress` by a previous process that crashed or was
            // force-quit mid-stream, before any IPC command can observe it.
            let turn_store = TurnStore::new(pool.clone());
            match turn_store.recover_orphaned_attempts() {
                Ok(0) => {}
                Ok(recovered) => {
                    log::warn!(
                        "[chat] recovered {recovered} orphaned turn attempt(s) from a previous session"
                    );
                }
                Err(e) => {
                    log::error!("[chat] failed to recover orphaned turn attempts: {e}");
                    return Err(e.into());
                }
            }

            // Register typed domain stores for conversation history IPC commands.
            // Each store wraps the same Arc<SqlitePool> so all stores share one connection.
            app.manage(ConversationStore::new(pool.clone()));
            app.manage(MessageStore::new(pool.clone()));
            app.manage(ArtifactStore::new(pool.clone()));
            app.manage(FtsStore::new(pool.clone()));
            app.manage(RetentionStore::new(pool.clone()));
            app.manage(turn_store);

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
            ipc::artifacts::artifact_get,
            ipc::artifacts::artifact_dismiss,
            ipc::history::history_list,
            ipc::history::history_get,
            ipc::history::history_delete,
            ipc::history::history_search,
            ipc::privacy::privacy_set_provider_key,
            ipc::privacy::privacy_get_credential_status,
            ipc::privacy::privacy_clear_provider_key,
            ipc::files::files_open_dialog,
            ipc::files::files_read_token,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed to start");
}
