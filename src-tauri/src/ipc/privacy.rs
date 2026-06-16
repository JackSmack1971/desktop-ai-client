use crate::app_state::AppState;
use crate::security::{command_policy, secrets};
use crate::telemetry::audit_log::{self, AuditEntry};

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivacyError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("unsupported provider: {0}")]
    UnsupportedProvider(String),
    #[error("credential store error: {0}")]
    CredentialStoreError(String),
    #[error("policy violation: {0}")]
    PolicyViolation(String),
}

impl From<command_policy::PolicyError> for PrivacyError {
    fn from(value: command_policy::PolicyError) -> Self {
        match value {
            command_policy::PolicyError::UnauthorizedWindow(msg) => {
                PrivacyError::UnauthorizedWindow(msg)
            }
            command_policy::PolicyError::UnknownCommand(msg) => PrivacyError::PolicyViolation(msg),
        }
    }
}

impl From<secrets::SecretsError> for PrivacyError {
    fn from(value: secrets::SecretsError) -> Self {
        match value {
            secrets::SecretsError::NotConfigured(msg)
            | secrets::SecretsError::StorageError(msg)
            | secrets::SecretsError::LockPoisoned(msg) => PrivacyError::CredentialStoreError(msg),
        }
    }
}

fn provider_from_string(provider: String) -> Result<secrets::ProviderId, PrivacyError> {
    match provider.as_str().parse::<secrets::ProviderId>() {
        Ok(provider) => Ok(provider),
        Err(_) => Err(PrivacyError::UnsupportedProvider(provider)),
    }
}

fn audit_result(app_handle: &tauri::AppHandle, window_label: &str, command: &str, ok: bool) {
    let status = if ok { "ok" } else { "error" };
    let _ = audit_log::write_audit_entry(app_handle, AuditEntry::new(command, window_label, status));
}

#[tauri::command]
pub async fn privacy_set_provider_key(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    _state: tauri::State<'_, AppState>,
    provider: String,
    key: String,
) -> Result<(), PrivacyError> {
    let result = (|| {
        command_policy::policy_check("privacy_set_provider_key", window.label())?;
        let provider_id = provider_from_string(provider)?;
        secrets::store_provider_key(provider_id.account_label(), &key).map_err(PrivacyError::from)
    })();
    audit_result(&app_handle, window.label(), "privacy_set_provider_key", result.is_ok());
    result
}

#[tauri::command]
pub async fn privacy_get_credential_status(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider: String,
) -> Result<secrets::CredentialStatus, PrivacyError> {
    let result = (|| {
        command_policy::policy_check("privacy_get_credential_status", window.label())?;
        let provider_id = provider_from_string(provider)?;
        Ok(secrets::get_credential_status(&state, provider_id))
    })();
    audit_result(&app_handle, window.label(), "privacy_get_credential_status", result.is_ok());
    result
}

#[tauri::command]
pub async fn privacy_clear_provider_key(
    window: tauri::Window,
    app_handle: tauri::AppHandle,
    _state: tauri::State<'_, AppState>,
    provider: String,
) -> Result<(), PrivacyError> {
    let result = (|| {
        command_policy::policy_check("privacy_clear_provider_key", window.label())?;
        let provider_id = provider_from_string(provider)?;
        secrets::delete_provider_key(provider_id.account_label()).map_err(PrivacyError::from)
    })();
    audit_result(&app_handle, window.label(), "privacy_clear_provider_key", result.is_ok());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_error_serializes_code_field() {
        let json = serde_json::to_string(&PrivacyError::UnsupportedProvider("x".into())).unwrap();
        assert!(json.contains("UNSUPPORTED_PROVIDER"), "json={json}");
    }

    #[test]
    fn privacy_set_returns_unit_type() {
        fn assert_result_type(_: Result<(), PrivacyError>) {}
        assert_result_type(Ok(()));
    }
}
