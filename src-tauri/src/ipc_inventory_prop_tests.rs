use crate::ipc::inventory::{compiled_command_allowlist, registered_commands_from_main_rs};
use crate::ipc::inventory::tests::temp_env_guard;
use proptest::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_temp_file(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let count = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("desktop-ai-client-{label}-{stamp}-{count}.rs"))
}

fn write_text(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}

fn command_name_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z][a-z0-9_]{0,15}").unwrap()
}

fn allowlist_token_strategy() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[a-z][a-z0-9_]{0,12}").unwrap()
}

fn handler_indent_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just(""), Just("    "), Just("        ")]
}

fn allowlist_separator_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just(","), Just(" "), Just("\n"), Just(", "), Just(",\n")]
}

fn allowlist_case_strategy() -> impl Strategy<Value = (Vec<String>, String)> {
    prop::collection::vec(allowlist_token_strategy(), 0..8).prop_flat_map(|tokens| {
        let separator_count = tokens.len().saturating_sub(1);
        (
            Just(tokens),
            prop::collection::vec(allowlist_separator_strategy(), separator_count),
            prop_oneof![Just(""), Just(" "), Just("\n"), Just("  ")],
            prop_oneof![Just(""), Just(" "), Just("\n"), Just("  ")],
        )
            .prop_map(|(tokens, separators, leading_ws, trailing_ws)| {
                let rendered = if tokens.is_empty() {
                    format!("{leading_ws}{trailing_ws}")
                } else {
                    let mut joined = String::new();
                    for (index, token) in tokens.iter().enumerate() {
                        if index > 0 {
                            joined.push_str(separators[index - 1]);
                        }
                        joined.push_str(token);
                    }
                    format!("{leading_ws}{joined}{trailing_ws}")
                };

                (tokens, rendered)
            })
    })
}

fn render_generate_handler(commands: &[String], indent: &str) -> String {
    let mut rendered = String::from(
        "fn main() {\n    tauri::Builder::default().invoke_handler(tauri::generate_handler![\n",
    );

    for command in commands {
        rendered.push_str(indent);
        rendered.push_str("ipc::chat::");
        rendered.push_str(command);
        rendered.push_str(",\n");
    }

    rendered.push_str("    ]);\n}\n");
    rendered
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

proptest! {
    #[test]
    fn registered_commands_from_main_rs_round_trips_generated_handler_block(
        commands in prop::collection::vec(command_name_strategy(), 0..8),
        indent in handler_indent_strategy(),
    ) {
        let path = unique_temp_file("inventory-handler-round-trip");
        write_text(&path, &render_generate_handler(&commands, indent));

        let parsed = registered_commands_from_main_rs(&path).unwrap();
        prop_assert_eq!(parsed, commands);
    }

    #[test]
    fn compiled_command_allowlist_preserves_tokens_across_delimiters(
        (tokens, rendered) in allowlist_case_strategy(),
    ) {
        let _guard = temp_env_guard();
        let _snapshot = AllowlistEnvSnapshot::set(&rendered);
        let parsed = compiled_command_allowlist();
        prop_assert_eq!(parsed, tokens);
    }
}
