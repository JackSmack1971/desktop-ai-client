pub mod artifacts;
/// Persistence, retention, and migration helpers.
///
/// All database access goes through typed backend commands; the frontend never
/// executes raw SQL. WAL mode and busy_timeout are enforced in SqlitePool.
///
/// Modules:
/// - `sqlite`     – Connection management and domain-typed stores
/// - `migrations` – Schema migration runner
/// - `fts`        – FTS5 full-text search helpers (future)
/// - `retention`  – Retention and deletion policy (future)
/// - `artifacts`  – Artifact preview storage and detection
/// - `turns`      – Conversation transaction protocol: turns/turn_attempts
/// - `memory`     – Evidence-Gated Memory Engine (Phase 1, shadow mode)
pub mod fts;
pub mod memory;
pub mod migrations;
pub mod retention;
pub mod sqlite;
pub mod turns;
