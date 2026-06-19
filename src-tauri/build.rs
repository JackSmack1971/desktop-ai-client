use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct CommandInventoryFile {
    commands: Vec<CommandInventoryEntry>,
}

#[derive(serde::Deserialize)]
struct CommandInventoryEntry {
    name: String,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = resolve_workspace_root(&manifest_dir);
    let inventory_path = workspace_root
        .join("security")
        .join("command-inventory.toml");

    println!("cargo:rerun-if-changed={}", inventory_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root
            .join("security")
            .join("release-capabilities.toml")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src").join("main.rs").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("permissions").display()
    );
    println!("cargo:rerun-if-changed=build.rs");

    let inventory_raw = fs::read_to_string(&inventory_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", inventory_path.display()));
    let inventory: CommandInventoryFile = toml::from_str(&inventory_raw)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", inventory_path.display()));

    let allowlist = inventory
        .commands
        .iter()
        .map(|command| command.name.as_str())
        .collect::<Vec<_>>()
        .join(",");
    println!("cargo:rustc-env=TAURI_COMPILED_COMMAND_ALLOWLIST={allowlist}");

    tauri_build::build();
}

fn resolve_workspace_root(manifest_dir: &PathBuf) -> PathBuf {
    let workspace_root = manifest_dir.parent().unwrap().to_path_buf();
    let parent_inventory = workspace_root
        .join("security")
        .join("command-inventory.toml");
    if parent_inventory.exists() {
        return workspace_root;
    }

    let local_inventory = manifest_dir.join("security").join("command-inventory.toml");
    if local_inventory.exists() {
        return manifest_dir.clone();
    }

    workspace_root
}
