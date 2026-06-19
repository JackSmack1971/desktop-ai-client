use desktop_ai_client_lib::telemetry::release_evidence::collect_release_evidence;
use std::path::PathBuf;

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri lives under the workspace root")
        .to_path_buf();

    match collect_release_evidence(&workspace_root) {
        Ok(bundle) => {
            println!(
                "release evidence written to {}",
                bundle.evidence_dir.display()
            );
            println!(
                "inventory: {} command(s), clean={}",
                bundle.manifest.inventory_commands, bundle.manifest.inventory_clean
            );
            for category in &bundle.manifest.categories {
                println!("category: {} [{}]", category.name, category.status);
            }
        }
        Err(err) => {
            eprintln!("failed to collect release evidence: {err}");
            std::process::exit(1);
        }
    }
}
