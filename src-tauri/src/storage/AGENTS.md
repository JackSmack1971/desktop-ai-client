# storage/AGENTS.md

This subtree owns persistence and retention.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/privacy-boundaries.md` — stub.
3. `../../../docs/threat-model.md` — stub.
4. `../AGENTS.md`

## Purpose

Owns the single `SqlitePool` connection wrapper (WAL mode, `busy_timeout=5000`, `foreign_keys=ON`), the migration runner, and typed domain stores: `ConversationStore`, `MessageStore`, `ShellPreferenceStore`, `ArtifactStore`, `FtsStore`, `RetentionStore`. `backup.rs` is an explicit scaffold placeholder, out of scope for Phase 3.

## Contracts & Invariants

- Migrations are append-only and ascending by `id`, never reordered or modified after shipping. `schema_migrations` is bootstrapped inline in `run_migrations` and deliberately excluded from the `MIGRATIONS` slice (avoids a misleading "applied N" count and a dual-creation path).
- All persistence goes through the typed domain stores — IPC handlers must never call `SqlitePool::with_conn` directly.
- FTS5 `MATCH` queries always bind the user query as a parameter, never interpolate it into SQL text (mitigation T-03-04).
- `messages_fts` is an FTS5 external-content table kept in sync purely by SQL triggers (`messages_ai`/`messages_ad`/`messages_au`, migration `0003`) — there is no application-level sync code.
- Tables added after migration `0001` use `STRICT` typing + explicit `CHECK` constraints for enum-like text columns. The original `shell_preferences` table (migration `0001`) predates this convention and is intentionally not `STRICT` — don't "fix" it without a migration.
- `SqlitePool::with_conn` treats a poisoned mutex as an unrecoverable panic, not a typed error — deliberate, because post-poison connection state is unknown (fixed from a prior misleading-error bug, WR-02, commit `8c51d48`).
- `RetentionStore::delete_conversation` is a hard delete (no soft-delete tombstone), runs `PRAGMA wal_checkpoint(TRUNCATE)` after every delete (checkpoint failure is logged, non-fatal, decision D-14); `ON DELETE CASCADE` + the FTS trigger keep `messages_fts` in sync automatically.
- `RetentionStore::delete_conversation` on a nonexistent id is a no-op returning `Ok(())` — callers must not treat absence as an error.
- `FtsStore::search` treats zero matches as success (empty `Vec`), not `QueryReturnedNoRows`.
- Keep backups and retention policies separate from migrations; do not mix storage concerns with UI or provider logic.

## Pitfalls

- Migration SQL is interpolated into a `SAVEPOINT migration_{id}` string via `format!`, guarded only by a `debug_assert!` validating `id` is alphanumeric/underscore — compiled out in release builds. The code's own `SAFETY:` comment says explicitly: do not generalize this pattern to dynamic values.
- A duplicate `schema_migrations` entry inside the `MIGRATIONS` slice was a shipped bug (WR-01, commit `51410c0`) — that's why the tracking table is bootstrapped outside the slice today; don't reintroduce it as a migration entry.

