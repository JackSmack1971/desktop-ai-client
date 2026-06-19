use crate::app_state::AppState;
use crate::security::{command_policy, file_tokens};
use mime_guess::from_path;
use std::fs;
use std::path::Path;
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FilesError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("dialog cancelled by user")]
    Cancelled,
    #[error("token not found: {0}")]
    TokenNotFound(String),
    #[error("policy violation: {0}")]
    PolicyViolation(String),
    #[error("io error: {0}")]
    IoError(String),
}

impl From<command_policy::PolicyError> for FilesError {
    fn from(value: command_policy::PolicyError) -> Self {
        match value {
            command_policy::PolicyError::UnauthorizedWindow(msg) => {
                FilesError::UnauthorizedWindow(msg)
            }
            command_policy::PolicyError::UnknownCommand(msg) => FilesError::PolicyViolation(msg),
        }
    }
}

impl From<file_tokens::FileTokenError> for FilesError {
    fn from(value: file_tokens::FileTokenError) -> Self {
        match value {
            file_tokens::FileTokenError::NotFound(token) => FilesError::TokenNotFound(token),
            file_tokens::FileTokenError::LockPoisoned(msg) => FilesError::IoError(msg),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileTokenResponse {
    pub token_id: String,
    pub filename: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileReadResponse {
    pub token_id: String,
    pub filename: String,
    pub size: u64,
    pub mime_type: String,
    pub content: Vec<u8>,
}

fn basename(path: &Path) -> Result<String, FilesError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| FilesError::IoError("selected file does not have a valid filename".into()))
}

fn safe_metadata(path: &Path) -> Result<(String, u64, String), FilesError> {
    let filename = basename(path)?;
    let size = fs::metadata(path)
        .map_err(|e| FilesError::IoError(e.to_string()))?
        .len();
    let mime_type = from_path(path).first_or_octet_stream().to_string();
    Ok((filename, size, mime_type))
}

#[tauri::command]
pub fn files_open_dialog(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<FileTokenResponse, FilesError> {
    command_policy::policy_check("files_open_dialog", window.label())?;
    let selected = app_handle.dialog().file().blocking_pick_file();
    let selected = match selected {
        Some(file_path) => file_path,
        None => return Err(FilesError::Cancelled),
    };
    let path = selected
        .into_path()
        .map_err(|_| FilesError::IoError("selected file path is not supported".into()))?;
    let (filename, size, mime_type) = safe_metadata(&path)?;
    let token = file_tokens::mint_token(&state, path)?;
    Ok(FileTokenResponse {
        token_id: token.to_string(),
        filename,
        size,
        mime_type,
    })
}

#[tauri::command]
pub async fn files_read_token(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    token_id: String,
) -> Result<FileReadResponse, FilesError> {
    command_policy::policy_check("files_read_token", window.label())?;
    let token = token_id
        .parse::<Uuid>()
        .map_err(|_| FilesError::TokenNotFound(token_id.clone()))?;
    let path = file_tokens::resolve_token(&state, token)?;
    let (filename, size, mime_type) = safe_metadata(&path)?;
    let content = fs::read(&path).map_err(|e| FilesError::IoError(e.to_string()))?;
    Ok(FileReadResponse {
        token_id,
        filename,
        size,
        mime_type,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_error_serializes_code_field() {
        let json = serde_json::to_string(&FilesError::Cancelled).unwrap();
        assert!(json.contains("CANCELLED"), "json={json}");
    }

    #[test]
    fn file_token_response_serializes_expected_fields() {
        let response = FileTokenResponse {
            token_id: "id".into(),
            filename: "file.txt".into(),
            size: 12,
            mime_type: "text/plain".into(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("token_id"));
        assert!(json.contains("filename"));
        assert!(json.contains("size"));
        assert!(json.contains("mime_type"));
        assert!(!json.contains("path"));
    }

    #[test]
    fn basename_helper_uses_file_name_only() {
        let path = Path::new("C:/tmp/nested/example.txt");
        assert_eq!(basename(path).unwrap(), "example.txt");
    }
}
