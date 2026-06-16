# Command Inventory

The reviewed command inventory is the release gate source of truth for every custom Tauri command exposed by this app.

## Canonical Files

- `security/command-inventory.toml` records the reviewed command surface.
- `security/release-capabilities.toml` records the explicit release capability selection.
- `src-tauri/permissions/*.toml` records the permission grant identifiers used by the capability file.
- `src-tauri/capabilities/main.json` is the release-selected capability wrapper for the main window.

## Inventory Fields

Each command entry in `security/command-inventory.toml` records:

- `name` - the registered Rust command name.
- `module` - the owning backend module.
- `allowed_windows` - the windows allowed to call the command.
- `production` - whether the command is intended for release builds.
- `debug_only` - whether the command is debug-only and must be excluded from release.
- `argument_schema` - a plain-language summary of the IPC arguments.
- `sensitivity` - the data sensitivity class used by release review.
- `expected_capability` - the `allow-*` permission identifier that grants the command.
- `required_negative_tests` - the minimum negative cases the reviewer expects to exist.

## Release Split

- The release catalog is explicit rather than inferred from the folder layout.
- `main-window` is the only release capability for Phase 6.
- No dev-only capability files are currently selected; that split is recorded as an empty list in `security/release-capabilities.toml`.

## Verifier Contract

`cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory` checks:

- `tauri::generate_handler![...]` in `src-tauri/src/main.rs`
- the reviewed inventory in `security/command-inventory.toml`
- the build-time allowlist exported by `build.rs`
- the permission grant files in `src-tauri/permissions`
- the selected release capability file in `src-tauri/capabilities/main.json`

The verifier fails closed if any command is missing, extra, debug-only, or mismatched across those sources.
