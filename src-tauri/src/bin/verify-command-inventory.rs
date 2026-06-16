use desktop_ai_client_lib::ipc::inventory::{self, InventoryError};
use std::path::PathBuf;

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri lives under the workspace root")
        .to_path_buf();
    let paths = inventory::workspace_paths(&workspace_root);

    match inventory::verify_inventory(&paths) {
        Ok(report) if report.is_clean() => {
            for line in report.summary_lines() {
                println!("{line}");
            }
        }
        Ok(report) => {
            for line in report.summary_lines() {
                println!("{line}");
            }
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("inventory verifier failed: {err}");
            std::process::exit(match err {
                InventoryError::Mismatch(_) => 1,
                _ => 2,
            });
        }
    }
}
