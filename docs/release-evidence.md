# Release Evidence

The release evidence bundle is a source-controlled summary of implemented-path verification and the fixture families that back it.

## Bundle Layout

`cargo run --manifest-path src-tauri/Cargo.toml --bin collect-release-evidence` writes:

- `release-evidence/manifest.toml` - the machine-readable evidence manifest.
- `release-evidence/fixtures.toml` - the fixture-family index.
- `release-evidence/test-runs/cargo-test.log` - captured output from the crate test suite.
- `release-evidence/test-runs/verify-command-inventory.log` - captured output from the release gate verifier.
- `release-evidence/README.md` - a short human-readable summary.

## Evidence Categories

The manifest preserves the hardening-spec structure with these categories:

- security checks
- streaming tests
- database/storage evidence
- provider-routed evidence
- command-inventory verification
- artifact sandbox and accessibility evidence
- adversarial fixture coverage

Categories that are not yet fully implemented remain represented as partial or deferred structure instead of fabricated proof.

## Fixture Families

The collector indexes the implemented fixture families under `tests/fixtures/`:

- `adversarial-sse`
- `provider-drift`
- `fts-query-abuse`
- `srcdoc-escaping`
- `wal-recovery`
- `capability-drift`

The bundle records file paths for the families that contain real source-controlled fixtures and marks missing families as deferred.

## Contract

Release evidence must be reproducible, must cite real test output, and must not claim coverage for paths that are not implemented yet.
