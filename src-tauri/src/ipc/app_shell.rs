/// IPC commands for the workspace shell surface preference.
///
/// These commands form the typed boundary between the frontend and the
/// backend-owned shell state. The frontend must never read or write the
/// active surface through browser storage; all persistence goes through here.
///
/// Security: both commands validate the caller window label so capability
/// files are defense-in-depth rather than the sole enforcement layer.
///
/// Command inventory entry:
///   get_active_surface  – windows: ["main"], production: true, sensitivity: low
///   set_active_surface  – windows: ["main"], production: true, sensitivity: low
use crate::app_state::{AppState, Surface};
use crate::security::command_policy;
use crate::storage::sqlite::ShellPreferenceStore;

/// Error type returned to the frontend from shell IPC commands.
/// Variants are serialized as structured error objects, never raw Rust panics.
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShellError {
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("invalid surface: {0}")]
    InvalidSurface(String),
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
}

impl From<command_policy::PolicyError> for ShellError {
    fn from(value: command_policy::PolicyError) -> Self {
        match value {
            command_policy::PolicyError::UnauthorizedWindow(msg) => {
                ShellError::UnauthorizedWindow(msg)
            }
            command_policy::PolicyError::UnknownCommand(msg) => ShellError::UnauthorizedWindow(msg),
        }
    }
}

/// Returns the active surface stored in backend-owned state.
///
/// On startup the frontend calls this to hydrate the shell without
/// relying on browser storage or local fallbacks.
///
/// Lock ordering: shell lock is acquired first, then sqlite lock (inside
/// `store.load_active_surface()`). All callers must maintain this ordering
/// to prevent deadlock.
#[doc(hidden)]
pub async fn get_active_surface_inner(
    state: &AppState,
    store: &ShellPreferenceStore,
) -> Result<Surface, ShellError> {
    // Hold the shell lock for the entire check-read-write sequence so that
    // concurrent async invocations cannot both observe hydrated == false and
    // both issue a DB read, returning a stale value from the second caller.
    // Lock ordering: shell -> sqlite (ShellPreferenceStore acquires sqlite internally).
    let mut shell = state
        .shell
        .lock()
        .map_err(|e| ShellError::StorageError(format!("shell state lock poisoned: {e}")))?;

    if !shell.hydrated {
        // DB read while holding the shell lock; sqlite lock acquired inside here.
        let persisted = store
            .load_active_surface()
            .map_err(|e| ShellError::StorageError(e.to_string()))?;
        if let Some(persisted) = persisted {
            shell.active_surface = persisted;
        }
        shell.hydrated = true;
    }

    Ok(shell.active_surface.clone())
}

#[tauri::command]
pub async fn get_active_surface(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, ShellPreferenceStore>,
) -> Result<Surface, ShellError> {
    command_policy::policy_check("get_active_surface", window.label())?;
    get_active_surface_inner(&state, &store).await
}

/// Sets the active surface and persists it to backend-owned SQLite storage.
///
/// The frontend calls this whenever the user switches surfaces. The value
/// is never written to browser storage; it lives only in the backend.
#[doc(hidden)]
pub async fn set_active_surface_inner(
    state: &AppState,
    store: &ShellPreferenceStore,
    surface: Surface,
) -> Result<(), ShellError> {
    let mut shell = state
        .shell
        .lock()
        .map_err(|e| ShellError::StorageError(format!("shell state lock poisoned: {e}")))?;
    let previous_surface = shell.active_surface.clone();
    shell.active_surface = surface;

    if let Err(e) = store.save_active_surface(&shell.active_surface) {
        shell.active_surface = previous_surface;
        return Err(ShellError::StorageError(e.to_string()));
    }

    Ok(())
}

#[tauri::command]
pub async fn set_active_surface(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, ShellPreferenceStore>,
    surface: Surface,
) -> Result<(), ShellError> {
    command_policy::policy_check("set_active_surface", window.label())?;
    set_active_surface_inner(&state, &store, surface).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;
    use crate::storage::sqlite::SqlitePool;
    use rusqlite::Connection;
    use std::sync::Arc;

    fn migrated_pool() -> Arc<SqlitePool> {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        run_migrations(&conn, "0.0.0-test").unwrap();
        Arc::new(SqlitePool::from_connection(conn))
    }

    #[test]
    fn shell_error_serializes_with_code_field() {
        let err = ShellError::InvalidSurface("bad".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(
            json.contains("INVALID_SURFACE"),
            "expected SCREAMING_SNAKE_CASE code: {json}"
        );
    }

    #[test]
    fn policy_check_rejects_non_main_window_for_get_active_surface() {
        let err: ShellError = command_policy::policy_check("get_active_surface", "evil")
            .unwrap_err()
            .into();
        assert!(matches!(err, ShellError::UnauthorizedWindow(_)));
    }

    #[test]
    fn policy_check_rejects_non_main_window_for_set_active_surface() {
        let err: ShellError = command_policy::policy_check("set_active_surface", "evil")
            .unwrap_err()
            .into();
        assert!(matches!(err, ShellError::UnauthorizedWindow(_)));
    }

    #[test]
    fn get_active_surface_restores_surface_on_restart_without_shell_conversation_state() {
        let pool = migrated_pool();
        let store = ShellPreferenceStore::new(pool.clone());
        store.save_active_surface(&Surface::History).unwrap();

        let first_session = AppState::default();
        let first = tauri::async_runtime::block_on(get_active_surface_inner(
            &first_session,
            &store,
        ))
        .unwrap();
        assert_eq!(first, Surface::History);
        assert!(first_session
            .shell
            .lock()
            .expect("shell lock should not be poisoned")
            .hydrated);

        let second_session = AppState::default();
        let second = tauri::async_runtime::block_on(get_active_surface_inner(
            &second_session,
            &store,
        ))
        .unwrap();
        assert_eq!(second, Surface::History);
        assert!(second_session
            .shell
            .lock()
            .expect("shell lock should not be poisoned")
            .hydrated);
    }

    #[test]
    fn get_active_surface_fails_closed_for_corrupted_persisted_surface() {
        let pool = migrated_pool();
        pool.with_conn(|conn| {
            conn.execute(
                "INSERT INTO shell_preferences (id, active_surface, updated_at)
                 VALUES (1, 'not-a-real-surface', datetime('now'))",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let state = AppState::default();
        let store = ShellPreferenceStore::new(pool);
        let err = tauri::async_runtime::block_on(get_active_surface_inner(&state, &store))
            .unwrap_err();
        assert!(
            matches!(err, ShellError::StorageError(_)),
            "invalid persisted surface should fail closed"
        );
        assert!(
            !state
                .shell
                .lock()
                .expect("shell lock should not be poisoned")
                .hydrated,
            "failed hydration must not mark the shell ready"
        );
    }
}
