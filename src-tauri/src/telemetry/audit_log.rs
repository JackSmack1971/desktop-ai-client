use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use tauri::Manager;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AuditEntry {
    pub timestamp: String,
    pub command: String,
    pub window: String,
    pub status: String,
}

impl AuditEntry {
    pub fn new(command: &str, window: &str, status: &str) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            command: command.to_string(),
            window: window.to_string(),
            status: status.to_string(),
        }
    }
}

pub fn write_audit_entry(
    app_handle: &tauri::AppHandle,
    entry: AuditEntry,
) -> Result<(), io::Error> {
    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| io::Error::other(e.to_string()))?;
    fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("audit.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let line = serde_json::to_string(&entry).map_err(|e| io::Error::other(e.to_string()))?;
    writeln!(file, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_round_trips_json() {
        let entry = AuditEntry::new("privacy_set_provider_key", "main", "ok");
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "privacy_set_provider_key");
        assert_eq!(parsed.window, "main");
        assert_eq!(parsed.status, "ok");
    }

    #[test]
    fn audit_entry_contains_only_metadata_fields() {
        let entry = AuditEntry::new("privacy_set_provider_key", "main", "ok");
        let value = serde_json::to_value(entry).unwrap();
        let obj = value.as_object().expect("audit entry object");
        let keys: Vec<_> = obj.keys().cloned().collect();
        assert_eq!(keys.len(), 4);
        assert!(keys.contains(&"timestamp".to_string()));
        assert!(keys.contains(&"command".to_string()));
        assert!(keys.contains(&"window".to_string()));
        assert!(keys.contains(&"status".to_string()));
    }
}
