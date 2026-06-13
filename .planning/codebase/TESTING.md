# Testing

## Current state

The repository contains test directories and fixture buckets, but no actual tests were present in the snapshot.

## Observed test areas

- `tests/rust/`
- `tests/e2e/`
- `tests/security/`
- `tests/fixtures/adversarial-sse/`
- `tests/fixtures/hostile-renderer/`
- `tests/fixtures/provider-drift/`
- `tests/fixtures/sqlite-corruption/`

## Implications

- Test structure has been anticipated early
- Security- and provider-related regression tests are expected to matter
- No baseline coverage exists yet, so future implementation will need to define the first executable test contract

## Likely priority

1. Backend command and policy tests
2. Provider routing and transport tests
3. Storage and corruption resilience tests
4. Security and redaction tests
5. End-to-end desktop flow tests
