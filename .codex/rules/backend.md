---
name: backend
description: API design, provider routing, storage, IPC, and backend-owned desktop behavior.
paths:
  - "src-tauri/src/**/*.rs"
  - "src-tauri/tauri.conf.json"
  - "src-tauri/tauri.conf.*.json"
  - "src-tauri/capabilities/**/*.json"
  - "src-tauri/permissions/**/*.{json,toml}"
  - "src/lib/api/**/*.{ts,tsx}"
  - "src/**/*tauri*.ts"
  - "src/**/*ipc*.ts"
  - "migrations/**/*.sql"
  - "db/**/*.sql"
  - "database/**/*.sql"
  - "schema/**/*.sql"
  - "sql/**/*.sql"
  - "src/**/*.sql"
  - "packages/**/*.sql"
---

# Backend Rules

## Architecture and Ownership

- Keep backend-owned concerns backend-owned: command policy, provider routing, storage, and telemetry stay out of the renderer.
- Keep the Rust/Tauri bootstrap thin and move behavior into named modules.
- Keep provider selection deterministic and backend-owned.
- Make capability detection explicit and testable.
- Keep fallback behavior visible in code and docs.
- Keep streaming transport ordered and cancellable.
- Do not let provider drift silently change the active model, transport, or capability set.
- Keep provider secrets and routing metadata out of ordinary frontend windows.

## Provider Routing and Streaming

- Preserve the existing stream protocol invariants: stable `stream_id`, unique `attempt_id`, strictly increasing `sequence`, backend-selected `provider_id` and `model_id`, and hashes for capability, request, and partial output.
- Classify every stream failure as one of `provider_timeout`, `provider_connection_lost`, `provider_protocol_error`, `provider_rate_limited`, `provider_auth_failed`, `provider_policy_rejected`, `frontend_channel_closed`, `frontend_cancelled`, or `backend_shutdown`.
- Before the first delta, the backend may retry the same provider/model once, then apply normal visible fallback policy if allowed.
- After the first delta, the backend must not silently switch providers, models, transports, or capability sets.
- On partial failure, emit `failed_partial`, preserve partial text, stop the upstream request, and expose visible recovery choices.
- Never splice a fallback provider's continuation into the same assistant message unless the provider supports explicit resumable stream recovery.
- Cancel upstream provider requests when the frontend channel closes.
- Reject recovery requests whose `retry_token` does not match the original `stream_id` and `partial_hash`.
- Expire retry tokens after a short window.
- Never let stale stream events mutate the active conversation.

## Tauri Config and Packaging

- Set `build.devUrl`, `build.beforeDevCommand`, `build.beforeBuildCommand`, and `build.frontendDist` to match the package manager scripts in `package.json`.
- Keep local development URLs in development config and package release builds from `build.frontendDist`.
- Define every `app.windows[]` entry with a stable `label`, `title`, `width`, `height`, and `resizable` value.
- Keep plugin configuration under `plugins.<name>` in `src-tauri/tauri.conf.json`.
- Align Tauri v2 crate and npm package major versions across `src-tauri/Cargo.toml` and `package.json`.
- Configure `bundle.active`, `productName`, `version`, and at least 1 icon path before release packaging.
- Keep every icon referenced by `bundle.icon` under `src-tauri/icons/**/*` and verify the file exists in the repository.
- Configure `app.security.csp` when the app loads IPC, assets, fonts, images, styles, or remote endpoints.
- Record each external `connect-src` host in the release checklist before packaging.
- Store custom protocol and asset protocol scopes under `app.security.assetProtocol.scope` with app-owned allow entries.
- Run `pnpm tauri dev` or the package-manager equivalent after development config changes.
- Run `pnpm tauri build` or `cargo tauri build` before merging changes that touch `src-tauri/**`, bundle settings, CSP, capabilities, or permissions.

## IPC Commands and Shared State

- Define each frontend-callable Rust entry point with `#[tauri::command]` in `src-tauri/src/**/*.rs`.
- Register every command through `tauri::generate_handler![...]` in the active `tauri::Builder` setup.
- Return `Result<T, E>` from async commands where `T` serializes through `serde` and `E` converts to a frontend-safe error shape.
- Access shared backend resources through `tauri::State<'_, T>` and initialize each resource exactly once with `Builder::manage(...)`.
- Use async-compatible locks for state guards that live across `.await` points.
- Keep `std::sync::Mutex` guards scoped to synchronous commands or code paths that finish before the first `.await`.
- Serialize command DTOs with camelCase fields when TypeScript consumers read the response payload.
- Wrap each frontend `invoke` from `@tauri-apps/api/core` in a typed client function under `src/**/tauri*.ts` or `src/**/ipc*.ts`.
- Match TypeScript payload keys exactly to Rust command parameter names for every `invoke(commandName, payload)` call.
- Use `tauri::ipc::Channel<T>` for ordered progress, download, stream, or long-running task events.
- Tag channel event enums with an `event` discriminator and a `data` payload for TypeScript-safe handling.
- Include the target window label in logs for commands that accept `tauri::WebviewWindow` or emit through `AppHandle`.
- Add at least 1 Rust unit test or TypeScript integration test for every new command contract.

## Storage and SQLite

- Keep persistence behind typed backend commands.
- Treat schema changes, migrations, retention rules, and backup behavior as explicit review items.
- Keep FTS/search behavior aligned with the conversation model and retention policy.
- Add corruption and recovery coverage when storage behavior changes.
- Do not let the renderer issue raw SQL or bypass retention rules.
- Store SQLite migrations as ordered, immutable `.sql` files and record each applied filename in the migration table.
- Wrap every schema migration in an explicit `BEGIN IMMEDIATE; ... COMMIT;` block with a paired `ROLLBACK;` path in the runner.
- Enable `PRAGMA foreign_keys = ON;` on every database connection before preparing migration or query statements.
- For table rebuild migrations, create `new_<table>`, copy rows with explicit column lists, drop the old table, rename the new one, recreate indexes, triggers, and views, run `PRAGMA foreign_key_check;`, then commit.
- Add nullable columns with `ALTER TABLE ... ADD COLUMN` when the default is `NULL` or a constant literal accepted by SQLite.
- Add `NOT NULL` columns with an explicit non-NULL default and include a backfill statement in the same migration.
- Model parent keys for foreign keys as table-level `PRIMARY KEY` or `UNIQUE` constraints.
- Use `STRICT` tables for new domain tables that need rigid type checking and document each storage class.
- Use `INTEGER PRIMARY KEY` for rowid-backed identifiers and `WITHOUT ROWID` for key-value or composite-primary-key tables that benefit from primary-key storage.
- Recreate every index, trigger, and view affected by a rebuilt table in the same migration file.
- Prepare every user-input query as a parameterized statement using SQLite bind placeholders.
- Bind application values through the driver API and keep SQL text constant across executions.
- Select explicit columns in application queries and reserve `SELECT *` for one-off inspection scripts.
- Give every multi-statement write batch an explicit transaction boundary.
- Add composite indexes for production queries with multiple AND-connected equality predicates and verify the intended index with `EXPLAIN QUERY PLAN`.
- Prefer one composite index over both `(a)` and `(a, b)` prefix indexes when the longer index covers both cases.
- Run `EXPLAIN QUERY PLAN` for every new production SELECT, UPDATE, or DELETE statement that filters or joins persisted tables.
- Treat `EXPLAIN QUERY PLAN` output as debugging evidence and verify runtime behavior through test fixtures or benchmarks.
- Run `PRAGMA optimize;` after migration batches that create or drop indexes and after bulk data loads.
- Include `ORDER BY` on every paginated query and pair it with deterministic cursor or `LIMIT`/`OFFSET` semantics documented in the query file.
- Use conflict clauses intentionally and document the selected behavior in a SQL comment next to the statement.
- After every migration that uses foreign keys, run `PRAGMA foreign_key_check;`.
- After every migration that rewrites rows or changes constraints, run `PRAGMA integrity_check;`.
- Add a rollback rehearsal or restore-from-backup rehearsal for migrations containing `DROP TABLE`, `DROP COLUMN`, or table-rebuild steps before release.

