# Release Evidence

Generated at: 2026-06-19T23:40:14.378333600+00:00

## Inventory

- registered commands: 15
- inventory status: clean
- release capabilities: main-window

## Runs

- cargo test: cargo test --manifest-path src-tauri/Cargo.toml (passed)
- verify-command-inventory: cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory (passed)

## Categories

- security checks: implemented
- streaming tests: implemented
- database/storage evidence: implemented
- provider-routed evidence: implemented
- command-inventory verification: implemented
- artifact sandbox and accessibility evidence: partial
- adversarial fixture coverage: implemented

## Fixture Families

- tests/fixtures/adversarial-sse: deferred
- tests/fixtures/provider-drift: deferred
- tests/fixtures/fts-query-abuse: deferred
- tests/fixtures/srcdoc-escaping: deferred
- tests/fixtures/wal-recovery: deferred
- tests/fixtures/capability-drift: deferred
