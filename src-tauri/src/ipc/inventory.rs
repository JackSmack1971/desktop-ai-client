use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("toml parse error in {path}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("json parse error in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("inventory mismatch: {0}")]
    Mismatch(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInventoryFile {
    pub schema_version: u32,
    pub reviewed_at: String,
    pub source: String,
    pub release_capabilities: String,
    pub compiled_allowlist: String,
    pub commands: Vec<CommandInventoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInventoryEntry {
    pub name: String,
    pub module: String,
    pub allowed_windows: Vec<String>,
    pub production: bool,
    pub debug_only: bool,
    pub argument_schema: String,
    pub sensitivity: String,
    pub expected_capability: String,
    pub required_negative_tests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCapabilitiesFile {
    pub schema_version: u32,
    pub selected_capabilities: Vec<CapabilitySelection>,
    pub dev_only_capabilities: Vec<CapabilitySelection>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySelection {
    pub identifier: String,
    pub path: String,
    pub windows: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionManifest {
    pub permission: Vec<PermissionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionEntry {
    pub identifier: String,
    pub description: String,
    pub commands: PermissionCommands,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCommands {
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityFile {
    pub identifier: String,
    pub windows: Vec<String>,
    pub permissions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReport {
    pub inventory_commands: Vec<String>,
    pub registered_commands: Vec<String>,
    pub compiled_allowlist: Vec<String>,
    pub permission_commands: Vec<String>,
    pub permission_identifiers: Vec<String>,
    pub capability_permissions: Vec<String>,
    pub release_capabilities: Vec<String>,
    pub issues: Vec<String>,
}

impl InventoryReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("registered commands: {}", self.registered_commands.len()),
            format!("inventory commands: {}", self.inventory_commands.len()),
            format!("compiled allowlist: {}", self.compiled_allowlist.len()),
            format!("permission grants: {}", self.permission_commands.len()),
            format!(
                "permission identifiers: {}",
                self.permission_identifiers.len()
            ),
            format!(
                "capability permissions: {}",
                self.capability_permissions.len()
            ),
            format!(
                "release capabilities: {}",
                self.release_capabilities.join(", ")
            ),
        ];

        if self.issues.is_empty() {
            lines.push("status: clean".to_string());
        } else {
            lines.push(format!("status: {} issue(s)", self.issues.len()));
            lines.extend(self.issues.iter().map(|issue| format!("issue: {issue}")));
        }

        lines
    }
}

pub fn workspace_paths(workspace_root: impl AsRef<Path>) -> InventoryPaths {
    let root = workspace_root.as_ref();
    InventoryPaths {
        inventory: root.join("security").join("command-inventory.toml"),
        release_capabilities: root.join("security").join("release-capabilities.toml"),
        main_rs: root.join("src-tauri").join("src").join("main.rs"),
        permissions_dir: root.join("src-tauri").join("permissions"),
        capability_file: root
            .join("src-tauri")
            .join("capabilities")
            .join("main.json"),
    }
}

#[derive(Debug, Clone)]
pub struct InventoryPaths {
    pub inventory: PathBuf,
    pub release_capabilities: PathBuf,
    pub main_rs: PathBuf,
    pub permissions_dir: PathBuf,
    pub capability_file: PathBuf,
}

pub fn load_command_inventory(
    path: impl AsRef<Path>,
) -> Result<CommandInventoryFile, InventoryError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| InventoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| InventoryError::Toml {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_release_capabilities(
    path: impl AsRef<Path>,
) -> Result<ReleaseCapabilitiesFile, InventoryError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| InventoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| InventoryError::Toml {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_main_capability(path: impl AsRef<Path>) -> Result<CapabilityFile, InventoryError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| InventoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| InventoryError::Json {
        path: path.to_path_buf(),
        source,
    })
}

pub fn load_permission_manifest(
    path: impl AsRef<Path>,
) -> Result<PermissionManifest, InventoryError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| InventoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| InventoryError::Toml {
        path: path.to_path_buf(),
        source,
    })
}

#[derive(Debug, Clone)]
pub struct PermissionGrant {
    pub identifier: String,
    pub command: String,
}

pub fn registered_commands_from_main_rs(
    path: impl AsRef<Path>,
) -> Result<Vec<String>, InventoryError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| InventoryError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let start = raw.find("tauri::generate_handler![").ok_or_else(|| {
        InventoryError::Mismatch("tauri::generate_handler![ block not found".into())
    })?;
    let block_start = start + "tauri::generate_handler![".len();
    let block_end = raw[block_start..].find(']').ok_or_else(|| {
        InventoryError::Mismatch("closing bracket for generate_handler! block not found".into())
    })? + block_start;
    let block = &raw[block_start..block_end];

    let mut commands = Vec::new();
    for raw_entry in block.split(',') {
        let entry = raw_entry.trim();
        if entry.is_empty() {
            continue;
        }
        let Some(name) = entry.rsplit("::").next() else {
            return Err(InventoryError::Mismatch(format!(
                "could not parse registered command entry: {entry}"
            )));
        };
        let name = name.trim();
        if !name.is_empty() {
            commands.push(name.to_string());
        }
    }
    Ok(commands)
}

pub fn compiled_command_allowlist() -> Vec<String> {
    let value = std::env::var("TAURI_COMPILED_COMMAND_ALLOWLIST")
        .or_else(|_| std::env::var("TAURI_COMMAND_ALLOWLIST"))
        .unwrap_or_default();
    value
        .split(|c| c == ',' || c == '\n' || c == ' ')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub fn permission_grants_from_dir(
    permissions_dir: impl AsRef<Path>,
) -> Result<Vec<PermissionGrant>, InventoryError> {
    let permissions_dir = permissions_dir.as_ref();
    let mut collected = Vec::new();
    let mut entries = fs::read_dir(permissions_dir)
        .map_err(|source| InventoryError::Io {
            path: permissions_dir.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| InventoryError::Io {
            path: permissions_dir.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let manifest = load_permission_manifest(&path)?;
        for permission in manifest.permission {
            let identifier = permission.identifier;
            for command in permission.commands.allow {
                collected.push(PermissionGrant {
                    identifier: identifier.clone(),
                    command,
                });
            }
        }
    }

    collected.sort_by(|left, right| {
        left.identifier
            .cmp(&right.identifier)
            .then_with(|| left.command.cmp(&right.command))
    });
    collected.dedup_by(|left, right| {
        left.identifier == right.identifier && left.command == right.command
    });
    Ok(collected)
}

pub fn capability_permissions_from_file(
    capability_path: impl AsRef<Path>,
) -> Result<Vec<String>, InventoryError> {
    let capability = load_main_capability(capability_path)?;
    let mut permissions = Vec::new();
    for value in capability.permissions {
        if let Some(permission) = value.as_str() {
            if permission.starts_with("allow-") {
                permissions.push(permission.to_string());
            }
        }
    }
    permissions.sort();
    permissions.dedup();
    Ok(permissions)
}

fn sorted_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let set: BTreeSet<String> = values.into_iter().collect();
    set.into_iter().collect()
}

fn diff_report(
    left: &[String],
    right: &[String],
    left_name: &str,
    right_name: &str,
) -> Vec<String> {
    let left_set: HashSet<_> = left.iter().collect();
    let right_set: HashSet<_> = right.iter().collect();
    let missing_from_right = left_set
        .difference(&right_set)
        .map(|s| format!("{s} missing from {right_name}"))
        .collect::<Vec<_>>();
    let extra_in_right = right_set
        .difference(&left_set)
        .map(|s| format!("{s} extra in {right_name}"))
        .collect::<Vec<_>>();
    let mut lines = Vec::new();
    if !missing_from_right.is_empty() {
        lines.push(format!(
            "{left_name} has {} item(s) not present in {right_name}: {}",
            missing_from_right.len(),
            missing_from_right.join(", ")
        ));
    }
    if !extra_in_right.is_empty() {
        lines.push(format!(
            "{right_name} has {} item(s) not present in {left_name}: {}",
            extra_in_right.len(),
            extra_in_right.join(", ")
        ));
    }
    lines
}

pub fn verify_inventory(paths: &InventoryPaths) -> Result<InventoryReport, InventoryError> {
    let inventory = load_command_inventory(&paths.inventory)?;
    let release_caps = load_release_capabilities(&paths.release_capabilities)?;
    let registered_commands = registered_commands_from_main_rs(&paths.main_rs)?;
    let compiled_allowlist = compiled_command_allowlist();
    let permission_grants = permission_grants_from_dir(&paths.permissions_dir)?;
    let capability_permissions = capability_permissions_from_file(&paths.capability_file)?;

    let inventory_commands =
        sorted_unique(inventory.commands.iter().map(|entry| entry.name.clone()));
    let inventory_expected_capabilities = sorted_unique(
        inventory
            .commands
            .iter()
            .map(|entry| entry.expected_capability.clone()),
    );
    let registered_commands = sorted_unique(registered_commands);
    let compiled_allowlist = sorted_unique(compiled_allowlist);
    let permission_commands =
        sorted_unique(permission_grants.iter().map(|grant| grant.command.clone()));
    let permission_identifiers = sorted_unique(
        permission_grants
            .iter()
            .map(|grant| grant.identifier.clone()),
    );
    let capability_permissions = sorted_unique(capability_permissions);
    let release_capability_ids = sorted_unique(
        release_caps
            .selected_capabilities
            .iter()
            .map(|entry| entry.identifier.clone()),
    );

    let mut issues = Vec::new();
    issues.extend(diff_report(
        &inventory_commands,
        &registered_commands,
        "inventory",
        "registered handler",
    ));
    issues.extend(diff_report(
        &inventory_commands,
        &compiled_allowlist,
        "inventory",
        "compiled allowlist",
    ));
    issues.extend(diff_report(
        &inventory_commands,
        &permission_commands,
        "inventory",
        "permission files",
    ));
    issues.extend(diff_report(
        &inventory_expected_capabilities,
        &permission_identifiers,
        "inventory expected capabilities",
        "permission identifiers",
    ));
    issues.extend(diff_report(
        &permission_identifiers,
        &capability_permissions,
        "permission identifiers",
        "main capability",
    ));

    if release_capability_ids != vec!["main-window".to_string()] {
        issues.push(format!(
            "release capability selection must contain only main-window, found: {}",
            release_capability_ids.join(", ")
        ));
    }

    if !release_caps.dev_only_capabilities.is_empty() {
        issues
            .push("dev_only_capabilities must remain empty in the current release catalog".into());
    }

    for entry in &inventory.commands {
        if entry.debug_only {
            issues.push(format!("command {} is marked debug_only", entry.name));
        }
        if !entry.production {
            issues.push(format!("command {} is not marked production", entry.name));
        }
        if entry.allowed_windows != vec!["main".to_string()] {
            issues.push(format!(
                "command {} must be limited to the main window",
                entry.name
            ));
        }
        if entry.expected_capability != command_permission_name(&entry.name) {
            issues.push(format!(
                "command {} expected capability {} does not match allow-{}",
                entry.name, entry.expected_capability, entry.name
            ));
        }
    }

    Ok(InventoryReport {
        inventory_commands,
        registered_commands,
        compiled_allowlist,
        permission_commands,
        permission_identifiers,
        capability_permissions,
        release_capabilities: release_capability_ids,
        issues,
    })
}

pub fn command_permission_name(command: &str) -> String {
    format!("allow-{}", command.replace('_', "-"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn unique_temp_dir(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("desktop-ai-client-{label}-{stamp}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_text(path: &Path, text: &str) {
        fs::write(path, text).unwrap();
    }

    fn temp_env_guard() -> MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    struct AllowlistEnvSnapshot {
        compiled: Option<String>,
        legacy: Option<String>,
    }

    impl AllowlistEnvSnapshot {
        fn set(value: &str) -> Self {
            let compiled = std::env::var("TAURI_COMPILED_COMMAND_ALLOWLIST").ok();
            let legacy = std::env::var("TAURI_COMMAND_ALLOWLIST").ok();
            std::env::set_var("TAURI_COMPILED_COMMAND_ALLOWLIST", value);
            std::env::remove_var("TAURI_COMMAND_ALLOWLIST");
            Self { compiled, legacy }
        }
    }

    impl Drop for AllowlistEnvSnapshot {
        fn drop(&mut self) {
            match &self.compiled {
                Some(value) => std::env::set_var("TAURI_COMPILED_COMMAND_ALLOWLIST", value),
                None => std::env::remove_var("TAURI_COMPILED_COMMAND_ALLOWLIST"),
            }

            match &self.legacy {
                Some(value) => std::env::set_var("TAURI_COMMAND_ALLOWLIST", value),
                None => std::env::remove_var("TAURI_COMMAND_ALLOWLIST"),
            }
        }
    }

    fn sample_inventory_toml() -> String {
        r#"
schema_version = 1
reviewed_at = "2026-06-15T23:00:00Z"
source = "src-tauri/src/main.rs"
release_capabilities = "security/release-capabilities.toml"
compiled_allowlist = "TAURI_COMPILED_COMMAND_ALLOWLIST"

[[commands]]
name = "chat_send"
module = "ipc::chat"
allowed_windows = ["main"]
production = true
debug_only = false
argument_schema = "messages, model, conversation_id, max_completion_tokens, temperature, attachments, channel"
sensitivity = "high"
expected_capability = "allow-chat-send"
required_negative_tests = ["rejects non-main window", "never accepts api_key"]

[[commands]]
name = "chat_cancel"
module = "ipc::chat"
allowed_windows = ["main"]
production = true
debug_only = false
argument_schema = "request_id"
sensitivity = "low"
expected_capability = "allow-chat-cancel"
required_negative_tests = ["rejects non-main window", "rejects unknown request id"]
"#
        .trim()
        .to_string()
    }

    #[test]
    fn command_inventory_round_trip() {
        let dir = unique_temp_dir("inventory-round-trip");
        let inventory_path = dir.join("command-inventory.toml");
        write_text(&inventory_path, &sample_inventory_toml());
        let parsed = load_command_inventory(&inventory_path).unwrap();
        assert_eq!(parsed.commands.len(), 2);
        assert_eq!(parsed.commands[0].name, "chat_send");
    }

    #[test]
    fn registered_handler_parser_extracts_commands() {
        let dir = unique_temp_dir("inventory-main");
        let main_rs = dir.join("main.rs");
        write_text(
            &main_rs,
            r#"
fn main() {
    tauri::Builder::default().invoke_handler(tauri::generate_handler![
        ipc::chat::chat_send,
        ipc::chat::chat_cancel,
    ]);
}
"#,
        );
        let commands = registered_commands_from_main_rs(&main_rs).unwrap();
        assert_eq!(
            commands,
            vec!["chat_send".to_string(), "chat_cancel".to_string()]
        );
    }

    #[test]
    fn command_inventory_detects_missing_command() {
        let dir = unique_temp_dir("inventory-missing");
        let workspace = dir.join("workspace");
        fs::create_dir_all(workspace.join("security")).unwrap();
        fs::create_dir_all(workspace.join("src-tauri/src")).unwrap();
        fs::create_dir_all(workspace.join("src-tauri/permissions")).unwrap();
        fs::create_dir_all(workspace.join("src-tauri/capabilities")).unwrap();
        write_text(
            &workspace.join("security/command-inventory.toml"),
            &sample_inventory_toml(),
        );
        write_text(
            &workspace.join("security/release-capabilities.toml"),
            r#"
schema_version = 1
notes = "release uses main-window only"

[[selected_capabilities]]
identifier = "main-window"
path = "src-tauri/capabilities/main.json"
windows = ["main"]
status = "release"

[[dev_only_capabilities]]
identifier = "none"
path = ""
windows = []
status = "deferred"
"#,
        );
        write_text(
            &workspace.join("src-tauri/src/main.rs"),
            r#"
fn main() {
    tauri::Builder::default().invoke_handler(tauri::generate_handler![
        ipc::chat::chat_send,
    ]);
}
"#,
        );
        write_text(
            &workspace.join("src-tauri/permissions/chat.toml"),
            r#"
[[permission]]
identifier = "allow-chat-send"
description = "Permits submitting a chat prompt and receiving streamed responses through the main window."

[permission.commands]
allow = ["chat_send"]
"#,
        );
        write_text(
            &workspace.join("src-tauri/capabilities/main.json"),
            r#"
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-window",
  "windows": ["main"],
  "permissions": ["allow-chat-send"]
}
"#,
        );
        let _guard = temp_env_guard();
        let _snapshot = AllowlistEnvSnapshot::set("chat_send");
        let report = verify_inventory(&workspace_paths(&workspace)).unwrap();
        assert!(!report.is_clean());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("registered handler")));
    }

    #[test]
    fn command_inventory_detects_extra_command() {
        let dir = unique_temp_dir("inventory-extra");
        let workspace = dir.join("workspace");
        fs::create_dir_all(workspace.join("security")).unwrap();
        fs::create_dir_all(workspace.join("src-tauri/src")).unwrap();
        fs::create_dir_all(workspace.join("src-tauri/permissions")).unwrap();
        fs::create_dir_all(workspace.join("src-tauri/capabilities")).unwrap();
        write_text(
            &workspace.join("security/command-inventory.toml"),
            &sample_inventory_toml(),
        );
        write_text(
            &workspace.join("security/release-capabilities.toml"),
            r#"
schema_version = 1
notes = "release uses main-window only"
dev_only_capabilities = []

[[selected_capabilities]]
identifier = "main-window"
path = "src-tauri/capabilities/main.json"
windows = ["main"]
status = "release"
"#,
        );
        write_text(
            &workspace.join("src-tauri/src/main.rs"),
            r#"
fn main() {
    tauri::Builder::default().invoke_handler(tauri::generate_handler![
        ipc::chat::chat_send,
        ipc::chat::chat_cancel,
        ipc::files::files_open_dialog,
    ]);
}
"#,
        );
        write_text(
            &workspace.join("src-tauri/permissions/chat.toml"),
            r#"
[[permission]]
identifier = "allow-chat-send"
description = "Permits submitting a chat prompt and receiving streamed responses through the main window."

[permission.commands]
allow = ["chat_send"]

[[permission]]
identifier = "allow-chat-cancel"
description = "Permits cancelling an in-flight chat stream from the main window."

[permission.commands]
allow = ["chat_cancel"]
"#,
        );
        write_text(
            &workspace.join("src-tauri/permissions/files.toml"),
            r#"
[[permission]]
identifier = "allow-files-open-dialog"
description = "Permits opening the native file picker and returning only backend-minted tokens plus safe metadata."

[permission.commands]
allow = ["files_open_dialog"]
"#,
        );
        write_text(
            &workspace.join("src-tauri/capabilities/main.json"),
            r#"
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-window",
  "windows": ["main"],
  "permissions": ["allow-chat-send", "allow-chat-cancel", "allow-files-open-dialog"]
}
"#,
        );
        let _guard = temp_env_guard();
        let _snapshot = AllowlistEnvSnapshot::set("chat_send,chat_cancel,files_open_dialog");
        let report = verify_inventory(&workspace_paths(&workspace)).unwrap();
        assert!(!report.is_clean());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("registered handler")));
    }

    #[test]
    fn release_catalog_stays_explicit() {
        let dir = unique_temp_dir("inventory-release");
        let workspace = dir.join("workspace");
        fs::create_dir_all(workspace.join("security")).unwrap();
        write_text(
            &workspace.join("security/release-capabilities.toml"),
            r#"
schema_version = 1
notes = "release uses main-window only"
dev_only_capabilities = []

[[selected_capabilities]]
identifier = "main-window"
path = "src-tauri/capabilities/main.json"
windows = ["main"]
status = "release"
"#,
        );
        let release =
            load_release_capabilities(workspace.join("security/release-capabilities.toml"))
                .unwrap();
        assert_eq!(release.selected_capabilities.len(), 1);
        assert!(release.dev_only_capabilities.is_empty());
        assert_eq!(release.selected_capabilities[0].identifier, "main-window");
    }

    #[test]
    fn reviewed_inventory_carries_negative_test_metadata() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let parsed =
            load_command_inventory(workspace_root.join("security/command-inventory.toml")).unwrap();
        assert!(parsed
            .commands
            .iter()
            .all(|command| !command.required_negative_tests.is_empty()));
        assert!(parsed
            .commands
            .iter()
            .all(|command| command.expected_capability == command_permission_name(&command.name)));
    }

    #[test]
    fn capability_permission_name_helper_matches_command_name() {
        assert_eq!(
            command_permission_name("files_read_token"),
            "allow-files-read-token"
        );
    }

    #[test]
    fn registered_commands_from_main_rs_handles_empty_handler_block() {
        let dir = unique_temp_dir("inventory-empty-handler");
        let path = dir.join("main.rs");
        write_text(
            &path,
            r#"
fn main() {
    tauri::Builder::default().invoke_handler(tauri::generate_handler![
    ]);
}
"#,
        );

        let commands = registered_commands_from_main_rs(&path).unwrap();
        assert!(commands.is_empty());
    }

    #[test]
    fn compiled_command_allowlist_ignores_separator_noise() {
        let _guard = temp_env_guard();
        let _snapshot =
            AllowlistEnvSnapshot::set("  chat_send,\n, history_list  files_open_dialog  ");

        let allowlist = compiled_command_allowlist();
        assert_eq!(
            allowlist,
            vec![
                "chat_send".to_string(),
                "history_list".to_string(),
                "files_open_dialog".to_string(),
            ]
        );
    }
}
