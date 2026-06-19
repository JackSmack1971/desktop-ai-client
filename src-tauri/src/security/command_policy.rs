#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
}

/// Only window in `tauri.conf.json`. Update this if a second window is added.
const ALLOWED_WINDOW: &str = "main";

const COMMANDS: &[&str] = &[
    "get_active_surface",
    "set_active_surface",
    "chat_send",
    "chat_cancel",
    "history_list",
    "history_get",
    "history_delete",
    "history_search",
    "privacy_set_provider_key",
    "privacy_get_credential_status",
    "privacy_clear_provider_key",
    "files_open_dialog",
    "files_read_token",
    "artifact_get",
    "artifact_dismiss",
];

pub fn policy_check(command: &str, window_label: &str) -> Result<(), PolicyError> {
    if !COMMANDS.contains(&command) {
        return Err(PolicyError::UnknownCommand(command.to_string()));
    }
    if window_label == ALLOWED_WINDOW {
        Ok(())
    } else {
        Err(PolicyError::UnauthorizedWindow(window_label.to_string()))
    }
}

/// Names of every command registered in the policy table.
///
/// Exposed so `ipc::inventory::verify_inventory` can reconcile this table
/// against `security/command-inventory.toml`, the registered handler list,
/// permission files, and capability files — closing the gap where this
/// registry could silently drift from the others (ARCH-002).
pub fn command_names() -> Vec<String> {
    POLICY.table.iter().map(|(name, _)| name.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_known_command_for_main_window() {
        assert!(policy_check("privacy_set_provider_key", "main").is_ok());
    }

    #[test]
    fn rejects_wrong_window() {
        assert!(matches!(
            policy_check("privacy_set_provider_key", "evil"),
            Err(PolicyError::UnauthorizedWindow(_))
        ));
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(matches!(
            policy_check("nonexistent_command", "main"),
            Err(PolicyError::UnknownCommand(_))
        ));
    }

    #[test]
    fn allows_artifact_commands_for_main_window() {
        assert!(policy_check("artifact_get", "main").is_ok());
        assert!(policy_check("artifact_dismiss", "main").is_ok());
    }

    #[test]
    fn serializes_code_field() {
        let json = serde_json::to_string(&PolicyError::UnauthorizedWindow("bad".into())).unwrap();
        assert!(json.contains("UNAUTHORIZED_WINDOW"), "json={json}");
    }
}
