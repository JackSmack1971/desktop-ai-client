use crate::app_state::AppState;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileTokenError {
    #[error("file token not found: {0}")]
    NotFound(String),
    #[error("file token lock poisoned: {0}")]
    LockPoisoned(String),
}

pub fn mint_token(state: &AppState, path: PathBuf) -> Result<Uuid, FileTokenError> {
    let token = Uuid::new_v4();
    let mut guard = state
        .file_tokens
        .lock()
        .map_err(|e| FileTokenError::LockPoisoned(e.to_string()))?;
    guard.insert(token, path);
    Ok(token)
}

pub fn resolve_token(state: &AppState, token: Uuid) -> Result<PathBuf, FileTokenError> {
    let guard = state
        .file_tokens
        .lock()
        .map_err(|e| FileTokenError::LockPoisoned(e.to_string()))?;
    guard
        .get(&token)
        .cloned()
        .ok_or_else(|| FileTokenError::NotFound(token.to_string()))
}

pub fn revoke_token(state: &AppState, token: Uuid) -> Result<(), FileTokenError> {
    let mut guard = state
        .file_tokens
        .lock()
        .map_err(|e| FileTokenError::LockPoisoned(e.to_string()))?;
    guard.remove(&token);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;

    #[test]
    fn mint_resolve_revoke_round_trip() {
        let state = AppState::default();
        let path = PathBuf::from("C:/tmp/example.txt");
        let token = mint_token(&state, path.clone()).expect("mint should succeed");
        assert_eq!(resolve_token(&state, token).expect("resolve"), path);
        revoke_token(&state, token).expect("revoke should succeed");
        assert!(matches!(
            resolve_token(&state, token),
            Err(FileTokenError::NotFound(_))
        ));
    }

    #[test]
    fn mint_produces_distinct_tokens() {
        let state = AppState::default();
        let first = mint_token(&state, PathBuf::from("a.txt")).expect("first mint");
        let second = mint_token(&state, PathBuf::from("b.txt")).expect("second mint");
        assert_ne!(first, second);
        let guard = state.file_tokens.lock().expect("lock");
        assert_eq!(guard.len(), 2);
    }

    #[test]
    fn unknown_token_returns_not_found() {
        let state = AppState::default();
        let token = Uuid::new_v4();
        let err = resolve_token(&state, token).expect_err("missing token");
        assert!(matches!(err, FileTokenError::NotFound(_)));
        if let FileTokenError::NotFound(message) = err {
            assert_eq!(message, token.to_string());
        }
    }
}
