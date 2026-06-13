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

fn main() {
    tauri::Builder::default()
        // Register the opener plugin so the app can open external URLs
        // from the backend (not exposed as a raw frontend command).
        .plugin(tauri_plugin_opener::init())
        // Inject shared runtime state. The storage layer will initialize
        // the SQLite connection and run pending migrations as part of
        // setup in a later phase; the shell preference is seeded from
        // the stored value there.
        .manage(AppState::default())
        // Register backend-owned IPC commands. Only commands listed here
        // and in the reviewed command inventory are reachable from the
        // frontend. Keep `app.withGlobalTauri` false in tauri.conf.json
        // so frontend code must import the specific Tauri APIs it needs.
        .invoke_handler(tauri::generate_handler![
            ipc::app_shell::get_active_surface,
            ipc::app_shell::set_active_surface,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri application failed to start");
}
