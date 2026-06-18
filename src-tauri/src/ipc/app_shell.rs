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
            command_policy::PolicyError::UnknownCommand(msg) => {
                ShellError::UnauthorizedWindow(msg)
            }
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
#[tauri::command]
pub async fn get_active_surface(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, ShellPreferenceStore>,
) -> Result<Surface, ShellError> {
    command_policy::policy_check("get_active_surface", window.label())?;

    // Hold the shell lock for the entire check-read-write sequence so that
    // concurrent async invocations cannot both observe hydrated == false and
    // both issue a DB read, returning a stale value from the second caller.
    // Lock ordering: shell -> sqlite (ShellPreferenceStore acquires sqlite internally).
    let mut shell = state.shell.lock().map_err(|e| {
        ShellError::StorageError(format!("shell state lock poisoned: {e}"))
    })?;

    if !shell.hydrated {
        // DB read while holding the shell lock; sqlite lock acquired inside here.
        if let Ok(Some(persisted)) = store.load_active_surface() {
            shell.active_surface = persisted;
        }
        shell.hydrated = true;
    }

    Ok(shell.active_surface.clone())
}

/// Sets the active surface and persists it to backend-owned SQLite storage.
///
/// The frontend calls this whenever the user switches surfaces. The value
/// is never written to browser storage; it lives only in the backend.
#[tauri::command]
pub async fn set_active_surface(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, ShellPreferenceStore>,
    surface: Surface,
) -> Result<(), ShellError> {
    command_policy::policy_check("set_active_surface", window.label())?;

    // Persist to SQLite first so that a crash between the DB write and the
    // in-memory update leaves the stored value correct.
    store
        .save_active_surface(&surface)
        .map_err(|e| ShellError::StorageError(e.to_string()))?;

    let mut shell = state.shell.lock().map_err(|e| {
        ShellError::StorageError(format!("shell state lock poisoned: {e}"))
    })?;
    shell.active_surface = surface;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_error_serializes_with_code_field() {
        let err = ShellError::InvalidSurface("bad".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("INVALID_SURFACE"), "expected SCREAMING_SNAKE_CASE code: {json}");
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
}
