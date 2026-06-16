use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Shared runtime state managed by Tauri's state system.
///
/// AppState is initialized during `tauri::Builder` setup and injected into every
/// command handler via `tauri::State<'_, AppState>`. It must remain `Send + Sync`
/// so it can live across async command invocations.
///
/// Privacy invariant: AppState must never expose provider credentials, raw file
/// paths, or prompt content to the frontend. Those values are backend-only and must
/// not appear in IPC responses.
#[derive(Debug)]
pub struct AppState {
    /// Shell preferences managed by the backend. The frontend reads and writes
    /// these through typed IPC commands rather than browser storage.
    pub shell: Mutex<ShellState>,
    /// Per-request cancellation tokens for in-flight streaming chat requests.
    /// Keyed by `request_id` (UUID string). Cleaned up unconditionally after
    /// each request completes (success, error, or cancellation).
    pub active_requests: Mutex<HashMap<String, CancellationToken>>,
    /// Provider credentials held in-process behind a Mutex.
    /// Populated from environment variables at startup; never crosses IPC.
    pub secrets: Mutex<SecretsState>,
    /// Session-scoped file tokens mapped to backend-owned paths.
    ///
    /// This map is in-memory only, never persisted, and dropped on app quit.
    /// The lock is independent of shell/sqlite ordering and must never be held
    /// across an await point.
    pub file_tokens: Mutex<HashMap<Uuid, PathBuf>>,
}

/// Provider credential state, backend-owned and never exposed via IPC.
///
/// Phase 2: backed by OPENROUTER_API_KEY environment variable.
/// Phase 4: internals replaced with Stronghold/OS keychain; callers unchanged.
#[derive(Debug)]
pub struct SecretsState {
    pub openrouter_key: Option<secrecy::SecretString>,
}

impl Default for SecretsState {
    fn default() -> Self {
        let key = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .map(|v| secrecy::SecretString::new(v.into()));
        Self { openrouter_key: key }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            shell: Mutex::new(ShellState::default()),
            active_requests: Mutex::new(HashMap::new()),
            secrets: Mutex::new(SecretsState::default()),
            file_tokens: Mutex::new(HashMap::new()),
        }
    }
}

/// Workspace shell preferences owned by the backend.
#[derive(Debug, Clone, Default)]
pub struct ShellState {
    /// The last active surface the user was on. Persisted to SQLite and restored
    /// on startup so the shell opens to the same surface after a restart.
    pub active_surface: Surface,
    /// Whether the shell has been hydrated from SQLite on this session. Starts
    /// false; set true after the first DB consult so subsequent calls use the
    /// cached in-memory value instead of re-querying.
    pub hydrated: bool,
}

/// Named surfaces the shell can display. Adding a new surface here requires a
/// corresponding migration in storage/migrations.rs to keep the persisted value
/// valid.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Chat,
    History,
    Settings,
    Artifacts,
}

impl Default for Surface {
    fn default() -> Self {
        Surface::Chat
    }
}

impl std::fmt::Display for Surface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Surface::Chat => "chat",
            Surface::History => "history",
            Surface::Settings => "settings",
            Surface::Artifacts => "artifacts",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for Surface {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(Surface::Chat),
            "history" => Ok(Surface::History),
            "settings" => Ok(Surface::Settings),
            "artifacts" => Ok(Surface::Artifacts),
            other => Err(format!("unknown surface: {other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_round_trips_through_string() {
        for s in ["chat", "history", "settings", "artifacts"] {
            let parsed: Surface = s.parse().expect("valid surface");
            assert_eq!(parsed.to_string(), s);
        }
    }

    #[test]
    fn surface_rejects_unknown_value() {
        assert!("unknown_surface".parse::<Surface>().is_err());
    }

    #[test]
    fn surface_serializes_as_snake_case() {
        let val = serde_json::to_string(&Surface::Chat).unwrap();
        assert_eq!(val, r#""chat""#);
    }

    #[test]
    fn surface_default_is_chat() {
        let state = ShellState::default();
        assert_eq!(state.active_surface, Surface::Chat);
    }

    #[test]
    fn app_state_initializes_active_requests_empty() {
        let state = AppState::default();
        let requests = state.active_requests.lock().expect("lock should not be poisoned");
        assert!(requests.is_empty(), "active_requests must start empty");
    }

    #[test]
    fn app_state_initializes_file_tokens_empty() {
        let state = AppState::default();
        let file_tokens = state.file_tokens.lock().expect("lock should not be poisoned");
        assert!(file_tokens.is_empty(), "file_tokens must start empty");
    }
}
