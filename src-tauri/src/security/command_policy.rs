#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyError {
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
}

pub struct CommandPolicy {
    table: &'static [(&'static str, &'static [&'static str])],
}

impl CommandPolicy {
    pub const fn new() -> Self {
        Self {
            table: &[
                ("get_active_surface", &["main"]),
                ("set_active_surface", &["main"]),
                ("chat_send", &["main"]),
                ("chat_cancel", &["main"]),
                ("history_list", &["main"]),
                ("history_get", &["main"]),
                ("history_delete", &["main"]),
                ("history_search", &["main"]),
                ("privacy_set_provider_key", &["main"]),
                ("privacy_get_credential_status", &["main"]),
                ("privacy_clear_provider_key", &["main"]),
                ("files_open_dialog", &["main"]),
                ("files_read_token", &["main"]),
                ("artifact_get", &["main"]),
                ("artifact_dismiss", &["main"]),
            ],
        }
    }

    pub fn check(&self, command: &str, window_label: &str) -> Result<(), PolicyError> {
        let Some((_, labels)) = self.table.iter().find(|(name, _)| *name == command) else {
            return Err(PolicyError::UnknownCommand(command.to_string()));
        };
        if labels.iter().any(|label| *label == window_label) {
            Ok(())
        } else {
            Err(PolicyError::UnauthorizedWindow(window_label.to_string()))
        }
    }
}

pub static POLICY: CommandPolicy = CommandPolicy::new();

pub fn policy_check(command: &str, window_label: &str) -> Result<(), PolicyError> {
    POLICY.check(command, window_label)
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
