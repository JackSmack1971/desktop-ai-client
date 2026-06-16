/// Provider credential access for the backend-owned keychain boundary.
///
/// Phase 4 stores provider credentials in the OS keychain via `keyring`.
/// The legacy env-backed path remains available only behind `cfg(test)` or the
/// explicit `dev-env-secrets` feature for local development and migration.
///
/// Security invariant: the SecretString wrapper redacts the key in
/// Debug/Display output automatically. Never call .expose_secret() inside
/// log macros, error format strings, or IPC response fields.
use crate::app_state::AppState;
use std::str::FromStr;
#[cfg(test)]
use crate::app_state::SecretsState;
#[cfg(test)]
use secrecy::ExposeSecret;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

/// Typed provider identifier. Exhaustive for Phase 2 (only OpenRouter is wired).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderId {
    OpenRouter,
}

impl ProviderId {
    pub(crate) fn account_label(self) -> &'static str {
        match self {
            ProviderId::OpenRouter => "openrouter",
        }
    }
}

impl FromStr for ProviderId {
    type Err = SecretsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("openrouter") {
            Ok(ProviderId::OpenRouter)
        } else {
            Err(SecretsError::NotConfigured(format!(
                "provider {value:?} is not configured"
            )))
        }
    }
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
    #[error("provider credential storage error: {0}")]
    StorageError(String),
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

#[cfg(test)]
fn test_key_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
fn reset_test_key_store() {
    if let Ok(mut store) = test_key_store().lock() {
        store.clear();
    }
}

fn provider_from_str(provider: &str) -> Result<ProviderId, SecretsError> {
    ProviderId::from_str(provider)
}

fn keyring_entry(provider: ProviderId) -> Result<keyring::Entry, SecretsError> {
    keyring::Entry::new("desktop-ai-client", provider.account_label())
        .map_err(|e| SecretsError::StorageError(e.to_string()))
}

#[cfg(test)]
fn test_store_provider_key(provider: ProviderId, key: &str) -> Result<(), SecretsError> {
    let mut store = test_key_store()
        .lock()
        .map_err(|e| SecretsError::LockPoisoned(e.to_string()))?;
    store.insert(provider.account_label().to_string(), key.to_string());
    Ok(())
}

#[cfg(test)]
fn test_read_provider_key(provider: ProviderId) -> Result<secrecy::SecretString, SecretsError> {
    let store = test_key_store()
        .lock()
        .map_err(|e| SecretsError::LockPoisoned(e.to_string()))?;
    match store.get(provider.account_label()) {
        Some(value) => Ok(secrecy::SecretString::new(value.clone().into())),
        None => Err(SecretsError::NotConfigured(format!(
            "{:?} API key is not configured",
            provider
        ))),
    }
}

#[cfg(test)]
fn test_delete_provider_key(provider: ProviderId) -> Result<(), SecretsError> {
    let mut store = test_key_store()
        .lock()
        .map_err(|e| SecretsError::LockPoisoned(e.to_string()))?;
    store.remove(provider.account_label());
    Ok(())
}

#[cfg(any(test, feature = "dev-env-secrets"))]
fn env_provider_key(provider: ProviderId) -> Option<secrecy::SecretString> {
    let env_name = match provider {
        ProviderId::OpenRouter => "OPENROUTER_API_KEY",
    };
    std::env::var(env_name)
        .ok()
        .map(|v| secrecy::SecretString::new(v.into()))
}

/// Store a provider key in the OS keychain.
pub fn store_provider_key(provider: &str, key: &str) -> Result<(), SecretsError> {
    let provider = provider_from_str(provider)?;
    #[cfg(test)]
    {
        return test_store_provider_key(provider, key);
    }
    #[cfg(not(test))]
    {
        let entry = keyring_entry(provider)?;
        entry
            .set_password(key)
            .map_err(|e| SecretsError::StorageError(e.to_string()))
    }
}

/// Read a provider key from the OS keychain or the dev/test env fallback.
pub fn read_provider_key(provider: &str) -> Result<secrecy::SecretString, SecretsError> {
    let provider = provider_from_str(provider)?;
    #[cfg(test)]
    {
        return test_read_provider_key(provider);
    }
    #[cfg(not(test))]
    {
        let entry = keyring_entry(provider)?;
        match entry.get_password() {
            Ok(password) => Ok(secrecy::SecretString::new(password.into())),
            Err(keyring::Error::NoEntry) => {
                #[cfg(any(test, feature = "dev-env-secrets"))]
                if let Some(secret) = env_provider_key(provider) {
                    return Ok(secret);
                }
                Err(SecretsError::NotConfigured(format!(
                    "{:?} API key is not configured",
                    provider
                )))
            }
            Err(e) => {
                #[cfg(any(test, feature = "dev-env-secrets"))]
                if let Some(secret) = env_provider_key(provider) {
                    return Ok(secret);
                }
                Err(SecretsError::StorageError(e.to_string()))
            }
        }
    }
}

/// Delete a provider key from the OS keychain.
pub fn delete_provider_key(provider: &str) -> Result<(), SecretsError> {
    let provider = provider_from_str(provider)?;
    #[cfg(test)]
    {
        return test_delete_provider_key(provider);
    }
    #[cfg(not(test))]
    {
        let entry = keyring_entry(provider)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretsError::StorageError(e.to_string())),
        }
    }
}

/// Return a cloned `SecretString` for the requested provider.
///
/// The keychain is consulted first. In production, any keychain failure is
/// treated as `NotConfigured` to fail closed. The env fallback is compiled only
/// in test/dev builds behind `dev-env-secrets`.
pub fn get_provider_key(
    _state: &AppState,
    provider: ProviderId,
) -> Result<secrecy::SecretString, SecretsError> {
    let label = provider.account_label();
    match read_provider_key(label) {
        Ok(secret) => Ok(secret),
        Err(SecretsError::NotConfigured(msg)) => {
            #[cfg(any(test, feature = "dev-env-secrets"))]
            if let Some(secret) = env_provider_key(provider) {
                return Ok(secret);
            }
            Err(SecretsError::NotConfigured(msg))
        }
        Err(SecretsError::StorageError(_msg)) => {
            #[cfg(any(test, feature = "dev-env-secrets"))]
            if let Some(secret) = env_provider_key(provider) {
                return Ok(secret);
            }
            Err(SecretsError::NotConfigured(format!(
                "{:?} API key is not configured",
                provider
            )))
        }
        Err(err) => Err(err),
    }
}

/// Return the configuration status for a provider without revealing the key.
pub fn get_credential_status(_state: &AppState, provider: ProviderId) -> CredentialStatus {
    let label = provider.account_label();
    match read_provider_key(label) {
        Ok(_) => CredentialStatus::Configured,
        Err(SecretsError::NotConfigured(_)) => {
            #[cfg(any(test, feature = "dev-env-secrets"))]
            if env_provider_key(provider).is_some() {
                return CredentialStatus::Configured;
            }
            CredentialStatus::Missing
        }
        Err(SecretsError::StorageError(_)) => {
            #[cfg(any(test, feature = "dev-env-secrets"))]
            if env_provider_key(provider).is_some() {
                return CredentialStatus::Configured;
            }
            CredentialStatus::Missing
        }
        Err(_) => CredentialStatus::Missing,
    }
}

/// Helper to construct an AppState with a custom SecretsState for testing.
#[cfg(test)]
fn make_state_with_secrets(secrets: SecretsState) -> AppState {
    use std::path::PathBuf;
    use std::sync::Mutex;
    use uuid::Uuid;
    AppState {
        shell: Mutex::new(crate::app_state::ShellState::default()),
        active_requests: Mutex::new(HashMap::new()),
        secrets: Mutex::new(secrets),
        file_tokens: Mutex::new(HashMap::<Uuid, PathBuf>::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn test_guard() -> MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD.get_or_init(|| Mutex::new(())).lock().expect("test guard")
    }

    #[test]
    fn read_provider_key_returns_not_configured_when_missing() {
        let _guard = test_guard();
        reset_test_key_store();
        let result = read_provider_key("openrouter");
        assert!(
            matches!(result, Err(SecretsError::NotConfigured(_))),
            "expected NotConfigured error, got: {:?}",
            result
        );
    }

    #[test]
    fn store_then_read_round_trips_through_keychain_backend() {
        let _guard = test_guard();
        reset_test_key_store();
        store_provider_key("openrouter", "sk-test").expect("store should succeed");
        let key = read_provider_key("openrouter").expect("read should succeed");
        assert_eq!(key.expose_secret(), "sk-test");
    }

    #[test]
    fn delete_provider_key_is_idempotent_when_absent() {
        let _guard = test_guard();
        reset_test_key_store();
        delete_provider_key("openrouter").expect("delete should succeed even when absent");
    }

    #[test]
    fn get_credential_status_returns_missing_when_no_key() {
        let _guard = test_guard();
        reset_test_key_store();
        let state = make_state_with_secrets(SecretsState { openrouter_key: None });
        let status = get_credential_status(&state, ProviderId::OpenRouter);
        assert!(
            matches!(status, CredentialStatus::Missing),
            "expected Missing when key is None"
        );
    }

    #[test]
    fn get_credential_status_returns_configured_when_key_present() {
        let _guard = test_guard();
        reset_test_key_store();
        store_provider_key("openrouter", "test-key").expect("store should succeed");
        let state = make_state_with_secrets(SecretsState {
            openrouter_key: None,
        });
        let status = get_credential_status(&state, ProviderId::OpenRouter);
        assert!(
            matches!(status, CredentialStatus::Configured),
            "expected Configured when key is Some"
        );
    }

    #[test]
    fn get_provider_key_returns_not_configured_when_missing() {
        let _guard = test_guard();
        reset_test_key_store();
        let state = make_state_with_secrets(SecretsState { openrouter_key: None });
        let result = get_provider_key(&state, ProviderId::OpenRouter);
        assert!(
            matches!(result, Err(SecretsError::NotConfigured(_))),
            "expected NotConfigured error, got: {:?}",
            result
        );
    }
}
