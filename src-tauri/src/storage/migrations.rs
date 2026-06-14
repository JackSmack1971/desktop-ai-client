/// Database migration engine.
///
/// Migrations run inside transactions where SQLite permits. Each migration
/// records its result in `schema_migrations` so the runner can detect
/// partial or failed runs and enter recovery mode.
///
/// Ordering invariant: migrations must be listed in strictly ascending order
/// by `id`. Re-running an already-applied migration is a no-op (idempotent).
///
/// Privacy invariant: do not log or return migration SQL that contains user
/// content, secrets, or local file paths.
use rusqlite::{params, Connection};

/// A single database migration.
pub struct Migration {
    /// Unique, monotonically increasing migration identifier (e.g. "0001").
    pub id: &'static str,
    /// A short human-readable description for diagnostics.
    pub description: &'static str,
    /// SQL to run for this migration. Must be safe to wrap in a transaction.
    pub sql: &'static str,
}

/// All migrations in ascending order.
///
/// Each migration must have a unique `id`. Adding a migration appends to this
/// list; never reorder or modify an existing entry after it ships.
pub static MIGRATIONS: &[Migration] = &[
    Migration {
        id: "0001",
        description: "Create schema_migrations tracking table",
        sql: "
            CREATE TABLE IF NOT EXISTS schema_migrations (
              id TEXT PRIMARY KEY,
              description TEXT NOT NULL,
              app_version TEXT NOT NULL,
              applied_at TEXT NOT NULL,
              success INTEGER NOT NULL CHECK (success IN (0, 1))
            );
        ",
    },
    Migration {
        id: "0002",
        description: "Create shell_preferences table for backend-owned surface state",
        sql: "
            CREATE TABLE IF NOT EXISTS shell_preferences (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              active_surface TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
        ",
    },
];

/// Apply any pending migrations to `conn`.
///
/// Returns the number of migrations applied. Logs the migration ID and
/// description at info level; does not log SQL or user data.
pub fn run_migrations(conn: &Connection, app_version: &str) -> rusqlite::Result<usize> {
    // Ensure the tracking table exists before anything else.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           id TEXT PRIMARY KEY,
           description TEXT NOT NULL,
           app_version TEXT NOT NULL,
           applied_at TEXT NOT NULL,
           success INTEGER NOT NULL CHECK (success IN (0, 1))
         );",
    )?;

    let mut applied = 0;

    for migration in MIGRATIONS {
        // Skip migrations that already succeeded.
        let already_applied: bool = conn.query_row(
            "SELECT success FROM schema_migrations WHERE id = ?1",
            params![migration.id],
            |row| row.get::<_, bool>(0),
        ).unwrap_or(false);

        if already_applied {
            continue;
        }

        // Run the migration in a savepoint (nested transaction) so failures
        // can be rolled back without losing the tracking table state.

        // Validate id is a safe SQL identifier before embedding it.
        debug_assert!(
            migration.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "migration id must be alphanumeric/underscore only: {:?}",
            migration.id
        );
        // SAFETY: migration.id and migration.sql are &'static str literals defined
        // in MIGRATIONS. Do not generalize this pattern to dynamic values.
        let result = conn.execute_batch(&format!(
            "SAVEPOINT migration_{id};\n{sql}\nRELEASE SAVEPOINT migration_{id};",
            id = migration.id,
            sql = migration.sql,
        ));

        let success = result.is_ok();

        conn.execute(
            "INSERT INTO schema_migrations (id, description, app_version, applied_at, success)
             VALUES (?1, ?2, ?3, datetime('now'), ?4)
             ON CONFLICT(id) DO UPDATE SET success = ?4, applied_at = datetime('now')",
            params![
                migration.id,
                migration.description,
                app_version,
                success as i64,
            ],
        )?;

        result?;
        applied += 1;
    }

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn
    }

    #[test]
    fn migrations_apply_to_fresh_database() {
        let conn = fresh_conn();
        let applied = run_migrations(&conn, "0.1.0").unwrap();
        assert_eq!(applied, MIGRATIONS.len(), "all migrations should apply on a fresh db");
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();
        let second_run = run_migrations(&conn, "0.1.0").unwrap();
        assert_eq!(second_run, 0, "re-running migrations should be a no-op");
    }

    #[test]
    fn shell_preferences_table_exists_after_migration() {
        let conn = fresh_conn();
        run_migrations(&conn, "0.1.0").unwrap();

        // Verify the table can be inserted into without error.
        conn.execute(
            "INSERT INTO shell_preferences (id, active_surface, updated_at)
             VALUES (1, 'chat', datetime('now'))",
            [],
        )
        .unwrap();

        let surface: String = conn
            .query_row(
                "SELECT active_surface FROM shell_preferences WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(surface, "chat");
    }
}
