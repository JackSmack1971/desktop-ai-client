use crate::security::{artifact_sandbox, command_policy};
use crate::storage::artifacts::{ArtifactPreview, ArtifactStore};

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactError {
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
}

impl From<command_policy::PolicyError> for ArtifactError {
    fn from(value: command_policy::PolicyError) -> Self {
        match value {
            command_policy::PolicyError::UnauthorizedWindow(msg) => {
                ArtifactError::UnauthorizedWindow(msg)
            }
            command_policy::PolicyError::UnknownCommand(msg) => ArtifactError::StorageError(msg),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ArtifactResponse {
    pub artifact_id: String,
    pub content_type: crate::storage::artifacts::ArtifactContentType,
    pub srcdoc: String,
}

fn assert_main_window(window: &tauri::Window) -> Result<(), ArtifactError> {
    if window.label() != "main" {
        return Err(ArtifactError::UnauthorizedWindow(format!(
            "artifact commands require the main window, got {:?}",
            window.label()
        )));
    }
    Ok(())
}

#[tauri::command]
pub async fn artifact_get(
    window: tauri::Window,
    artifact_id: String,
    store: tauri::State<'_, ArtifactStore>,
) -> Result<ArtifactResponse, ArtifactError> {
    command_policy::policy_check("artifact_get", window.label())?;
    assert_main_window(&window)?;

    let preview: ArtifactPreview =
        store
            .get_artifact_preview(&artifact_id)
            .map_err(|e| match e {
                crate::storage::artifacts::ArtifactStoreError::NotFound(id) => {
                    ArtifactError::NotFound(id)
                }
                other => ArtifactError::StorageError(other.to_string()),
            })?;

    if preview.srcdoc.trim().is_empty() {
        return Err(ArtifactError::StorageError(
            "artifact preview could not be generated safely".into(),
        ));
    }

    let _ = artifact_sandbox::ARTIFACT_CSP;

    Ok(ArtifactResponse {
        artifact_id: preview.artifact_id,
        content_type: preview.content_type,
        srcdoc: preview.srcdoc,
    })
}

#[tauri::command]
pub async fn artifact_dismiss(
    window: tauri::Window,
    artifact_id: String,
    store: tauri::State<'_, ArtifactStore>,
) -> Result<(), ArtifactError> {
    command_policy::policy_check("artifact_dismiss", window.label())?;
    assert_main_window(&window)?;

    let _ = store
        .get_artifact_row(&artifact_id)
        .map_err(|e| ArtifactError::StorageError(e.to_string()))?
        .ok_or_else(|| ArtifactError::NotFound(artifact_id.clone()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_error_serializes_with_code_field() {
        let json = serde_json::to_string(&ArtifactError::NotFound("x".into())).unwrap();
        assert!(json.contains("NOT_FOUND"), "json={json}");
    }
}
