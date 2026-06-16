# Release Evidence

Generated at: 2026-06-16T04:04:08.634183800+00:00

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

- tests/fixtures/adversarial-sse: implemented
  - tests/fixtures/adversarial-sse/error-stream.txt
- tests/fixtures/provider-drift: implemented
  - tests/fixtures/provider-drift/openrouter.json
- tests/fixtures/fts-query-abuse: implemented
  - tests/fixtures/fts-query-abuse/query.txt
- tests/fixtures/srcdoc-escaping: implemented
  - tests/fixtures/srcdoc-escaping/payload.html
- tests/fixtures/wal-recovery: implemented
  - tests/fixtures/wal-recovery/recovery.sql
- tests/fixtures/capability-drift: implemented
  - tests/fixtures/capability-drift/main-window.json
