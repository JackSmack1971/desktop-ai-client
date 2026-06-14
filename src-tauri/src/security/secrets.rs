/// Provider credential access — thin Phase 2 stub.
///
/// Phase 2 backing: reads OPENROUTER_API_KEY from environment at startup and
/// holds it in AppState behind a Mutex. Phase 4 replaces internals with
/// Stronghold/OS keychain without changing callers.
///
/// Security invariant: the SecretString wrapper redacts the key in
/// Debug/Display output automatically. Never call .expose_secret() inside
/// log macros, error format strings, or IPC response fields.
use crate::app_state::{AppState, SecretsState};
use secrecy::ExposeSecret;

/// Typed provider identifier. Exhaustive for Phase 2 (only OpenRouter is wired).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderId {
    OpenRouter,
}

/// Errors returned by the secrets module.
///
/// Serialized as `{ code: "SCREAMING_SNAKE_CASE", message: string }` to match
/// the established IPC error shape.
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecretsError {
    #[error("provider key not configured: {0}")]
    NotConfigured(String),
    #[error("state lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Whether a provider credential is present (without revealing its value).
///
/// Used by the frontend to show a configuration status indicator without
/// ever receiving the credential itself.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CredentialStatus {
    Configured,
    Missing,
}

/// Return a cloned `SecretString` for the requested provider.
///
/// The lock is acquired, the secret is cloned into a new `SecretString`, and
/// the lock is released before this function returns. The caller must never
/// hold the returned secret across an await point where it is not needed.
///
/// Error cases:
/// - `SecretsError::NotConfigured` — key is absent (env var not set at startup).
/// - `SecretsError::LockPoisoned` — Mutex was poisoned by a panicking thread.
pub fn get_provider_key(
    state: &AppState,
    provider: ProviderId,
) -> Result<secrecy::SecretString, SecretsError> {
    let secrets = state
        .secrets
        .lock()
        .map_err(|e| SecretsError::LockPoisoned(e.to_string()))?;

    let key = match provider {
        ProviderId::OpenRouter => &secrets.openrouter_key,
    };

    match key {
        Some(k) => {
            // Expose the raw string, clone it, then re-wrap so the original
            // Mutex guard can be dropped immediately.
            let raw = k.expose_secret().to_string();
            drop(secrets); // release lock before returning
            Ok(secrecy::SecretString::new(raw.into()))
        }
        None => Err(SecretsError::NotConfigured(format!(
            "{:?} API key is not configured",
            provider
        ))),
    }
}

/// Return the configuration status for a provider without revealing the key.
///
/// The lock is held only for the duration of the option check.
pub fn get_credential_status(state: &AppState, provider: ProviderId) -> CredentialStatus {
    let Ok(secrets) = state.secrets.lock() else {
        return CredentialStatus::Missing;
    };
    let configured = match provider {
        ProviderId::OpenRouter => secrets.openrouter_key.is_some(),
    };
    drop(secrets);
    if configured {
        CredentialStatus::Configured
    } else {
        CredentialStatus::Missing
    }
}

/// Helper to construct an AppState with a custom SecretsState for testing.
#[cfg(test)]
fn make_state_with_secrets(secrets: SecretsState) -> AppState {
    use std::collections::HashMap;
    use std::sync::Mutex;
    AppState {
        shell: Mutex::new(crate::app_state::ShellState::default()),
        active_requests: Mutex::new(HashMap::new()),
        secrets: Mutex::new(secrets),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_credential_status_returns_missing_when_no_key() {
        let state = make_state_with_secrets(SecretsState { openrouter_key: None });
        let status = get_credential_status(&state, ProviderId::OpenRouter);
        assert!(
            matches!(status, CredentialStatus::Missing),
            "expected Missing when key is None"
        );
    }

    #[test]
    fn get_credential_status_returns_configured_when_key_present() {
        let state = make_state_with_secrets(SecretsState {
            openrouter_key: Some(secrecy::SecretString::new("test-key".to_string().into())),
        });
        let status = get_credential_status(&state, ProviderId::OpenRouter);
        assert!(
            matches!(status, CredentialStatus::Configured),
            "expected Configured when key is Some"
        );
    }

    #[test]
    fn get_provider_key_returns_not_configured_when_missing() {
        let state = make_state_with_secrets(SecretsState { openrouter_key: None });
        let result = get_provider_key(&state, ProviderId::OpenRouter);
        assert!(
            matches!(result, Err(SecretsError::NotConfigured(_))),
            "expected NotConfigured error, got: {:?}",
            result
        );
    }
}
