# Plan 005: Establish frontend test infra and cover the chat streaming/cancellation orchestration

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. This plan is larger than the others in this
> batch — treat each numbered step as a checkpoint; it is fine to commit
> after each one. When done, update the status row for this plan in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- package.json src-tauri/Cargo.toml src/lib/stores/chat.ts src-tauri/src/ipc/chat.rs src-tauri/src/providers/openrouter.rs src-tauri/src/storage/migrations.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: L
- **Risk**: MED
- **Depends on**: none (independent of plans 001-004, though it will make their behavior easier to regression-test in the future — no hard ordering requirement)
- **Category**: tests
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

This repo has zero frontend test files (no `*.test.ts`, no Vitest/Playwright config) and the chat streaming/cancellation/error orchestration — the single most complex and highest-risk piece of behavior in the app — has no test coverage above the SQL-storage layer. `chat_send`'s `#[cfg(test)]` block only tests JSON serialization and a pure string helper (`title_from_messages`); none of the actual streaming, cancellation, partial-persistence, or error-handling logic in `chat_send`/`chat_cancel`/`run_stream` is exercised. This is also why testing it has been hard: `providers::openrouter::stream_completion` makes a real `reqwest` HTTP call with no dev-dependency for mocking it, so there's currently no way to test the request/response boundary without hitting the live OpenRouter API.

`.claude/rules/testing.md` already mandates Vitest for `$lib/**` and "at least 1 test for every new `#[tauri::command]`" — the rules exist, the tooling to satisfy them doesn't. This plan builds both: the frontend test runner, and backend HTTP mocking, then uses them to cover the riskiest paths.

## Current state

- `package.json` scripts (lines 6-14): `dev`, `build`, `preview`, `check`, `check:watch`, `frontend:dev`, `frontend:build`, `tauri` — no `test` script, no Vitest in `devDependencies`.
- `src-tauri/Cargo.toml` has no `[dev-dependencies]` section at all (full file already read during audit — confirmed).
- `src/lib/stores/chat.ts` (full file, 245 lines) — the riskiest untested logic: `handleEvent` (lines 78-138, a switch over 5 `ChatEvent` variants), `sendMessage` (146-191), `cancelRequest` (200-211). Imports `chatSend`/`chatCancel` from `$lib/api/chat`.
- `src/lib/api/chat.ts` — wraps the Tauri `invoke`/`Channel` calls; this is the seam to mock in frontend tests (read this file yourself before Step 2 — it was not included in this plan's excerpts because its exact shape determines how you mock it, and guessing its shape risks writing tests against an API that doesn't exist).
- `src-tauri/src/ipc/chat.rs:559-723` — existing test module; the new backend tests in this plan are added to this same module, not a new file, to match the established one-test-module-per-source-file convention used everywhere else in this codebase (see `command_policy.rs`, `file_tokens.rs`, `fts.rs`, all of which keep tests in the same file as the code).
- `src-tauri/src/providers/openrouter.rs:54-90` (`stream_completion`) — hardcodes `OPENROUTER_BASE` (line 22, `"https://openrouter.ai/api/v1"`) directly into the request URL (line 71: `client.post(format!("{OPENROUTER_BASE}/chat/completions"))`). To make this mockable, the base URL needs to become an injectable parameter with the current constant as the production default.
- `src-tauri/src/storage/migrations.rs:140-199` (`run_migrations`) — wraps each migration in `SAVEPOINT`/`RELEASE SAVEPOINT` via `execute_batch`, records success/failure in `schema_migrations`, but has no test exercising a migration that actually fails (all 8 existing tests in this file are happy-path: fresh DB, idempotent re-run, schema checks).

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Type-check | `pnpm check` | exit 0 |
| Install new frontend deps | `pnpm install` | exit 0 |
| Run new frontend tests | `pnpm test` (added in Step 1) | all pass |
| Compile check backend | `cargo check --manifest-path src-tauri/Cargo.toml` | exit 0 |
| Compile check backend tests | `cargo check --manifest-path src-tauri/Cargo.toml --tests` | exit 0 |
| Run backend tests | `cargo test --manifest-path src-tauri/Cargo.toml` | all pass |

## Scope

**In scope** (the only files you should create/modify):
- `package.json` — add Vitest as a dev dependency and a `test` script.
- New file: `vitest.config.ts` (or extend `vite.config.ts` with a `test` block — prefer a separate `vitest.config.ts` if `vite.config.ts` already has SvelteKit-specific plugin config that Vitest needs reused; read `vite.config.ts` first to decide).
- New file: `src/lib/stores/chat.test.ts`
- `src-tauri/Cargo.toml` — add `mockito` (or `wiremock`, pick `mockito` for simplicity unless you find a reason it can't satisfy reqwest 0.13's TLS stack — check its README/docs for reqwest 0.13 compatibility before committing to it) under `[dev-dependencies]`.
- `src-tauri/src/providers/openrouter.rs` — parameterize the base URL.
- `src-tauri/src/ipc/chat.rs` — update the one call site that constructs `stream_completion`'s arguments (inside `run_stream`) to pass the production base URL explicitly; add new integration-style tests.
- `src-tauri/src/storage/migrations.rs` — add one failure-path test.

**Out of scope** (do NOT touch, even though related):
- Playwright / end-to-end browser tests — `.claude/rules/testing.md` mentions Playwright for hydrated UI flows, but standing up a full E2E harness is a much larger effort than this plan's scope; this plan only covers Vitest unit/store-level tests and backend integration tests.
- `src/lib/stores/history.ts`, `settings.ts`, `artifacts.ts`, `surface.ts` — also untested, but out of scope for this plan; `chat.ts` is the highest-risk one and the one this plan's audit specifically flagged. Add a follow-up plan for the others if this pattern proves out.
- Any change to `chat_send`/`chat_cancel`'s actual behavior — this plan only adds tests and the minimum plumbing (base-URL parameterization) needed to make them testable. Do not "fix" anything you notice while writing tests; report it instead as a new finding.
- CI wiring (`.github/workflows/`) — covered by `plans/008-add-ci-workflow.md`; this plan only makes `pnpm test` and `cargo test` runnable locally.

## Git workflow

- Branch: `advisor/005-add-test-coverage`
- Commit per step (7 commits), message style:
  - `chore(frontend): add vitest`
  - `test(chat): cover handleEvent and sendMessage in chatStore`
  - `chore(backend): add mockito dev-dependency`
  - `fix(providers): parameterize OpenRouter base URL for testing`
  - `test(chat): cover chat_send/chat_cancel against a mock provider`
  - `test(migrations): cover the failed-migration rollback path`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Add Vitest to the frontend

1. Read `vite.config.ts` to see its current plugin setup.
2. Run `pnpm add -D vitest @testing-library/svelte jsdom`. (Use `@testing-library/svelte` only if `chat.ts`'s store can be tested as a plain module without rendering a component — likely true here, since `chatStore` is a factory function exporting plain getters/methods, not a component. If you find you don't end up rendering any `.svelte` file in `chat.test.ts`, you may skip `@testing-library/svelte` and `jsdom` — try writing Step 2's tests first against plain Node, and only add these if a DOM/Svelte-runtime dependency turns out to be required.)
3. Add a `"test": "vitest run"` script to `package.json`.
4. Create `vitest.config.ts`:
   ```ts
   import { defineConfig } from 'vitest/config';
   import { svelte } from '@sveltejs/vite-plugin-svelte';

   export default defineConfig({
       plugins: [svelte()],
       test: {
           environment: 'node', // switch to 'jsdom' only if a test needs DOM APIs
       },
   });
   ```
   Adjust the plugin list to match whatever `vite.config.ts` actually uses — do not invent a different plugin set; copy what's already there.

**Verify**: `pnpm install` → exit 0. `pnpm test` → runs (0 test files yet is fine at this point, should not error).

### Step 2: Write `src/lib/stores/chat.test.ts`

First, read `src/lib/api/chat.ts` in full to learn the exact exported shape of `chatSend`/`chatCancel` and the `ChatEvent` type before writing mocks against it — do not guess its signature from this plan's excerpts of `chat.ts` alone.

Write tests covering, at minimum:
1. `sendMessage` appends a user message and an assistant placeholder, then calls `chatSend` with the right message list (excluding the placeholder).
2. Simulating an `Ack` event sets `requestId` and `canCancel` becomes true.
3. Simulating a `Delta` event after `Ack`: first delta flips `loading` to `false` and marks the message `streaming: true`; subsequent deltas append `text` to `content`.
4. Simulating a `Done` event: message becomes `status: 'complete'`, `streaming: false`; `streamingId`/`requestId` reset to `null`; `canCancel` becomes `false`.
5. Simulating an `Error` event with `code: 'CANCELLED'`: message becomes `status: 'incomplete'`.
6. Simulating an `Error` event with a different code: message becomes `status: 'error'`, store's `error` getter is non-null.
7. `cancelRequest` calls `chatCancel` with the current `requestId`; if `chatCancel` rejects, the rejection is swallowed (no throw) — matches the documented best-effort behavior at `chat.ts:200-211`.
8. **The regression test for `plans/001-guard-concurrent-chat-send.md`'s fix** (only if plan 001 has already landed — check `git log --oneline -- src/lib/stores/chat.ts` for a commit referencing it, or just read the current file: if `sendMessage` already has the `if (canCancel) return;` guard, write this test; if not, skip it and note in your plan-005 status update that this sub-test was deferred pending plan 001): calling `sendMessage` a second time while `canCancel` is true is a no-op (`messages.length` does not grow).

Mock `chatSend`/`chatCancel` from `$lib/api/chat` using Vitest's `vi.mock`. Since `chatSend` takes an `onEvent` callback (per `chat.ts:182`: `chatSend({ messages: apiMessages, onEvent: handleEvent })`), your mock implementation should capture that callback and let the test invoke it manually with synthetic `ChatEvent` objects to drive each scenario above.

**Verify**: `pnpm test` → all new tests in `chat.test.ts` pass. `pnpm check` → exit 0 (no type errors introduced by the test file).

### Step 3: Add `mockito` as a backend dev-dependency

In `src-tauri/Cargo.toml`, add:
```toml
[dev-dependencies]
mockito = "1"
```
Check `mockito`'s published docs/changelog for explicit reqwest-version compatibility notes before locking this in — `mockito` only needs to run an HTTP server and doesn't depend on reqwest itself, so this should be safe regardless of the app's reqwest version, but confirm rather than assume.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml --tests` → exit 0 (dependency resolves and the (currently still real-network) existing tests still compile).

### Step 4: Parameterize `stream_completion`'s base URL

In `src-tauri/src/providers/openrouter.rs`, change `stream_completion`'s signature to accept an explicit base URL parameter instead of using the `OPENROUTER_BASE` constant directly inside the function body:

```rust
pub async fn stream_completion(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &secrecy::SecretString,
    model: &str,
    messages: &[ProviderMessage],
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<reqwest::Response, String> {
    let body = ChatCompletionRequest { /* unchanged */ };
    let response = client
        .post(format!("{base_url}/chat/completions"))
        // ...rest unchanged
```

Keep `OPENROUTER_BASE` as the public constant — it becomes the value production call sites pass in, not something removed.

Update the one production call site in `src-tauri/src/ipc/chat.rs`'s `run_stream` (around line 410-417) to pass `crate::providers::openrouter::OPENROUTER_BASE` explicitly:
```rust
r = crate::providers::openrouter::stream_completion(
    &client,
    crate::providers::openrouter::OPENROUTER_BASE,
    api_key,
    model,
    &messages,
    max_completion_tokens,
    temperature,
) => { /* unchanged */ }
```

Update `openrouter.rs`'s own existing tests if any call `stream_completion` directly (check the existing `#[cfg(test)] mod tests` block at the bottom of the file — as read during recon, the existing tests only exercise `ChatCompletionRequest` serialization and the `DEFAULT_MODEL` constant, not `stream_completion` itself, so likely no existing test needs updating — confirm this yourself).

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0. `cargo test --manifest-path src-tauri/Cargo.toml --lib providers::openrouter` → all existing tests still pass.

### Step 5: Add `stream_completion` tests against a mock server, plus `chat_send`/`chat_cancel` integration tests

In `src-tauri/src/providers/openrouter.rs`'s test module, add:
```rust
#[tokio::test]
async fn stream_completion_returns_network_error_message_on_send_failure() {
    // Point at a URL with nothing listening to force a connection error.
    let client = reqwest::Client::new();
    let key = secrecy::SecretString::new("test-key".into());
    let result = stream_completion(&client, "http://127.0.0.1:1", &key, "test/model", &[], None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().starts_with("network error:"));
}

#[tokio::test]
async fn stream_completion_returns_http_status_on_non_2xx_response() {
    let mut server = mockito::Server::new_async().await;
    let _m = server.mock("POST", "/chat/completions").with_status(429).create_async().await;
    let client = reqwest::Client::new();
    let key = secrecy::SecretString::new("test-key".into());
    let result = stream_completion(&client, &server.url(), &key, "test/model", &[], None, None).await;
    assert_eq!(result.unwrap_err(), "HTTP 429");
}
```
Adjust the exact `mockito` API calls to whatever the locked `mockito = "1"` version's actual async API surface is (check its docs — the snippet above reflects mockito 1.x's `Server::new_async`/`mock(...).create_async()` pattern as of this plan's writing, but verify against the resolved version in `Cargo.lock` after Step 3).

Then, in `src-tauri/src/ipc/chat.rs`'s test module, add an integration-style test for the cancellation path (the highest-value untested branch): a test that drives `run_stream` directly (it's a private function in the same module, so the test module can call it without going through the full `#[tauri::command]` machinery) with a pre-cancelled `CancellationToken`, asserting the result is `Err("CANCELLED")` and a `ChatEvent::Error { code: "CANCELLED", .. }` was sent on the channel. Use a `tauri::ipc::Channel` test double if one is needed — check whether `Channel` can be constructed directly in a unit test in this Tauri version (`tauri = "2"`); if not, this confirms the audit's original observation that `chat_send`/`chat_cancel` are hard to test end-to-end without more infrastructure, and it's acceptable to scope this test down to calling `run_stream` with a mock HTTP response (via the same `mockito` server from the `openrouter.rs` tests) and asserting on the returned tuple `(result, accumulated_text, done_model)` rather than the channel send — whichever is actually constructible, prefer the more complete one.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --lib providers::openrouter ipc::chat` → all pass, including the new tests.

### Step 6: Add a migration-failure test

In `src-tauri/src/storage/migrations.rs`'s test module, add a test that deliberately injects a failing migration and asserts the savepoint rollback behavior described in the file's own doc comment actually holds:

```rust
#[test]
fn failed_migration_is_recorded_and_does_not_corrupt_the_connection() {
    let conn = fresh_conn();
    // Run the real migrations first so the schema_migrations table exists
    // and prior migrations are applied cleanly.
    run_migrations(&conn, "0.0.0-test").expect("baseline migrations should succeed");

    // Manually exercise the same SAVEPOINT/RELEASE pattern run_migrations uses,
    // with a deliberately invalid statement, to verify the connection is still
    // usable afterward (run_migrations itself only iterates the fixed MIGRATIONS
    // constant, so to test a failure without modifying that constant, replicate
    // its savepoint pattern directly against the same connection):
    let result = conn.execute_batch(
        "SAVEPOINT migration_test_fail;\n\
         CREATE TABLE this_is_fine (id TEXT);\n\
         THIS IS NOT VALID SQL;\n\
         RELEASE SAVEPOINT migration_test_fail;"
    );
    assert!(result.is_err(), "deliberately invalid SQL should fail");

    // The connection must still be usable for ordinary queries after the
    // failed/unreleased savepoint — confirms SQLite's implicit rollback of an
    // unreleased savepoint when no further action is taken on it.
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| row.get(0))
        .expect("connection should still be queryable after a failed savepoint");
    assert!(count > 0, "schema_migrations should still have the baseline rows");
}
```

This test directly verifies (or disproves) the claim made during this plan's audit: that an unreleased savepoint does not leave the connection permanently unusable. If this test fails (i.e., the connection really is left in a broken state), that is a more serious finding than originally assessed — STOP and report it rather than trying to fix `run_migrations`'s rollback handling under this test-coverage plan; that would need its own plan.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --lib storage::migrations` → all pass, including the new test.

## Test plan

(This plan's "test plan" is the steps above — each step's `#[test]`/`it(...)` additions are the deliverable, not a separate later phase.)

## Done criteria

- [ ] `pnpm test` exits 0 and runs at least 7 tests in `src/lib/stores/chat.test.ts`
- [ ] `pnpm check` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` exits 0, including new tests in `providers::openrouter`, `ipc::chat`, and `storage::migrations`
- [ ] `grep -n "mockito" src-tauri/Cargo.toml` shows it under `[dev-dependencies]`
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- `src/lib/api/chat.ts`'s actual exported shape doesn't match the `{ messages, onEvent }` call pattern assumed from `chat.ts:182` — read it fully before writing mocks.
- `mockito` turns out to be incompatible with the resolved `reqwest` 0.13 / TLS stack — switch to `wiremock` instead and note the substitution in your final report; do not silently fall back to live-network tests.
- The migration-failure test in Step 6 reveals the connection really is left unusable after a failed savepoint — stop, report the failing assertion, and do not attempt to fix `run_migrations` under this plan.
- `tauri::ipc::Channel` cannot be constructed in a unit test context at all and there's no documented test-double pattern — scope Step 5's `chat.rs` test down to `run_stream` only (not the full `#[tauri::command]`), as described, and note the gap rather than blocking the whole plan on it.

## Maintenance notes

- Once this lands, `pnpm test` and `cargo test` are real, runnable verification gates — `plans/008-add-ci-workflow.md` should wire both into CI as a natural next step (that plan is independent of this one but more valuable once this one lands).
- The `MAX_ATTACHMENT_BYTES`/non-text-rejection tests added by `plans/004-cap-attachment-size-and-type.md` and the `canCancel` guard test added by `plans/001-guard-concurrent-chat-send.md` both slot into the test files this plan creates — if those plans land after this one, their authors should add to `chat.test.ts`/`chat.rs`'s test module rather than creating new files.
- `src/lib/stores/history.ts`, `settings.ts`, `artifacts.ts` remain untested after this plan — flagged as a deliberate scope cut, not an oversight; worth a follow-up plan once this Vitest setup is proven out.
