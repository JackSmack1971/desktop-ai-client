# Testing Patterns

**Analysis Date:** 2026-06-13

## Test Framework

### Rust

**Runner:** Cargo's built-in test harness (`cargo test`)

**Run Commands:**
```bash
cargo test --workspace --all-targets   # Run all tests (unit + integration)
cargo test -p desktop-ai-client        # Run crate tests only
```

**Assertion style:** Standard `assert_eq!`, `assert!`, `.unwrap()` / `.expect()` with descriptive messages.

No `proptest`, `quickcheck`, or `tokio::test` async test attributes detected yet. All current tests are synchronous.

### Frontend

No test framework installed. `package.json` contains no `test` script, no Vitest, no Jest, no Playwright. `svelte-check` (`npm run check`) performs TypeScript and template type checking only — it is not a test runner.

---

## Test File Locations

### Rust Unit Tests (inline `#[cfg(test)]`)

| File | Test module | What is tested |
|------|-------------|----------------|
| `src-tauri/src/app_state.rs` | `mod tests` | `Surface` parse/display round-trip, unknown value rejection, serde snake_case serialization, `ShellState` default |
| `src-tauri/src/ipc/app_shell.rs` | `mod tests` | `ShellError` serde shape — verifies `SCREAMING_SNAKE_CASE` code field |
| `src-tauri/src/storage/migrations.rs` | `mod tests` | Fresh-DB migration apply, idempotent re-run, `shell_preferences` table usability |
| `src-tauri/src/storage/sqlite.rs` | `mod tests` | `ShellPreferenceStore` save/load round-trip, initial `None` when no row, overwrite |

### Rust Integration Tests

| File | Status | Notes |
|------|--------|-------|
| `tests/rust/app_shell.rs` | Stub only | Comment redirects to `src-tauri/tests/app_shell.rs`; actual integration tests not yet present |
| `tests/rust/.gitkeep` | Empty | Placeholder directory |

### Frontend Tests

None. `src/` contains no `*.test.ts`, `*.spec.ts`, or `*.test.svelte` files.

### E2E Tests

`tests/e2e/.gitkeep` — directory placeholder only. No framework configured (no Playwright, Cypress, or WebDriver setup).

### Security Tests

`tests/security/.gitkeep` — directory placeholder only. No test files present.

### Test Fixtures

All fixture directories are empty placeholders:
- `tests/fixtures/adversarial-sse/` — intended for hostile SSE stream fixtures
- `tests/fixtures/hostile-renderer/` — intended for hostile renderer behavior fixtures
- `tests/fixtures/provider-drift/` — intended for provider capability drift fixtures
- `tests/fixtures/sqlite-corruption/` — intended for SQLite corruption recovery fixtures

---

## What Is Covered

### Covered

- **`Surface` enum:** Parse from string, display to string, serde snake_case output, unknown value rejection, default value. (`src-tauri/src/app_state.rs`)
- **`ShellError` serde shape:** Confirms the IPC error boundary emits `SCREAMING_SNAKE_CASE` code tags. (`src-tauri/src/ipc/app_shell.rs`)
- **Migration engine:** Fresh-DB apply, idempotent re-run, schema correctness for `shell_preferences`. (`src-tauri/src/storage/migrations.rs`)
- **`ShellPreferenceStore`:** save, load, overwrite — round-trip against an in-memory SQLite pool. (`src-tauri/src/storage/sqlite.rs`)

### Not Covered

- **IPC command handlers** (`get_active_surface`, `set_active_surface`): require `tauri::Window` and `tauri::State` — no mock or integration test exists yet.
- **Window label enforcement** (`assert_main_window`): the unauthorized-window path is untested.
- **Provider routing and capabilities:** `src-tauri/src/providers/routing.rs`, `capabilities.rs`, `openrouter.rs`, `sse.rs` are all scaffold placeholders with no tests.
- **Security modules:** `src-tauri/src/security/` (redaction, file tokens, artifact sandbox, command policy) are all scaffold placeholders.
- **Telemetry modules:** `src-tauri/src/telemetry/` (audit log, release evidence) are scaffold placeholders.
- **Storage modules:** `src-tauri/src/storage/fts.rs`, `backup.rs`, `retention.rs` are scaffold placeholders.
- **Frontend store behavior:** `surfaceStore` hydrate, optimistic update, and rollback logic in `src/lib/stores/surface.ts` have no tests.
- **Svelte component behavior:** All components in `src/lib/components/` are untested.
- **E2E flows:** No end-to-end tests for any user journey.
- **Adversarial / hostile-renderer paths:** Fixture directories exist but no tests consume them.
- **SQLite corruption recovery:** Fixture directory exists but no tests exist.
- **SSE streaming and cancellation:** No tests.

---

## Test Patterns in Use

### In-Memory SQLite Helper

Tests that need storage create an in-memory connection directly:
```rust
fn fresh_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    conn
}
```

`SqlitePool::from_connection(conn)` is the production API for wrapping a test-prepared connection. See `src-tauri/src/storage/sqlite.rs`.

### Schema Inline for Unit Tests

Storage unit tests that need a table but do not want the full migration engine create the schema inline:
```rust
pool.with_conn(|conn| {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS shell_preferences (...);")
}).unwrap();
```

This keeps unit tests independent of migration ordering.

### Boundary / Failure Mode Testing

The `testing.md` rule requires testing failure modes, not just happy paths. Current tests cover:
- `Surface::from_str` rejecting unknown values
- `store.load_active_surface()` returning `None` on an empty table

Hostile-path coverage for IPC, provider routing, and security modules is missing.

---

## Coverage Gaps by Priority

| Area | Gap | Priority |
|------|-----|----------|
| IPC command handlers | `get_active_surface` and `set_active_surface` not tested end-to-end | High |
| Window label enforcement | Unauthorized window path untested | High |
| Frontend store | `surfaceStore` hydrate, optimistic update, rollback untested | High |
| Security / redaction | All modules are scaffold placeholders | High |
| Provider routing | All modules are scaffold placeholders | High |
| Storage (FTS, backup, retention) | All modules are scaffold placeholders | Medium |
| Telemetry | All modules are scaffold placeholders | Medium |
| E2E flows | No framework, no tests | Medium |
| Adversarial SSE fixtures | Directory exists, no tests consume them | Medium |
| SQLite corruption recovery | Directory exists, no tests consume them | Medium |
| Svelte components | No frontend test framework installed | Low (blocked on tooling) |

---

*Testing analysis: 2026-06-13*
