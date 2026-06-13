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

/// Returns the active surface stored in backend-owned state.
///
/// On startup the frontend calls this to hydrate the shell without
/// relying on browser storage or local fallbacks.
#[tauri::command]
pub async fn get_active_surface(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    store: tauri::State<'_, ShellPreferenceStore>,
) -> Result<Surface, ShellError> {
    assert_main_window(&window)?;

    // Prefer the in-memory value when already hydrated (avoids a DB read on
    // every re-render). On first call after launch the store loads from SQLite
    // and populates the in-memory state.
    let (current, hydrated) = {
        let shell = state.shell.lock().map_err(|e| {
            ShellError::StorageError(format!("shell state lock poisoned: {e}"))
        })?;
        (shell.active_surface.clone(), shell.hydrated)
    };

    // Consult the DB exactly once per session, guarded by an explicit flag
    // rather than default-value equality (which would fail when Chat is the
    // persisted value and the user is genuinely on Chat).
    if !hydrated {
        if let Ok(Some(persisted)) = store.load_active_surface() {
            let mut shell = state.shell.lock().map_err(|e| {
                ShellError::StorageError(format!("shell state lock poisoned: {e}"))
            })?;
            shell.active_surface = persisted.clone();
            shell.hydrated = true;
            return Ok(persisted);
        }
        // No persisted value — DB consulted; mark hydrated so we don't re-query.
        let mut shell = state.shell.lock().map_err(|e| {
            ShellError::StorageError(format!("shell state lock poisoned: {e}"))
        })?;
        shell.hydrated = true;
    }

    Ok(current)
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
    assert_main_window(&window)?;

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

/// Enforces that shell commands can only be invoked from the main window.
/// This is backend-side enforcement; the capability file is defense-in-depth.
fn assert_main_window(window: &tauri::Window) -> Result<(), ShellError> {
    if window.label() != "main" {
        return Err(ShellError::UnauthorizedWindow(format!(
            "shell commands require the main window, got {:?}",
            window.label()
        )));
    }
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
}
