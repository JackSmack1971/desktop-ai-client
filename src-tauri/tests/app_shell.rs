/// Shell preference persistence smoke tests.
///
/// These integration tests verify the backend-owned shell state round trip:
///   write active surface → read back → confirm value matches.
///
/// They also verify the startup hydration path — that on a database with a
/// previously stored preference the `load_active_surface` call returns the
/// stored value and that a fresh database returns `None` (so the shell falls
/// back to the default Chat surface).
///
/// No Tauri runtime or frontend is required to run these tests. They operate
/// directly against the Rust storage layer, proving the persistence behavior
/// independently of IPC.
///
/// Run with: cargo test --workspace --all-targets
use desktop_ai_client_lib::app_state::{AppState, Surface};
use desktop_ai_client_lib::ipc::app_shell::{get_active_surface_inner, set_active_surface_inner};
use desktop_ai_client_lib::storage::migrations::run_migrations;
use desktop_ai_client_lib::storage::sqlite::{ShellPreferenceStore, SqlitePool};
use rusqlite::Connection;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Barrier;
use std::thread;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Opens an in-memory SQLite connection with the required pragmas applied and
/// all schema migrations run.  Returns a pool ready for use by `ShellPreferenceStore`.
fn migrated_pool() -> Arc<SqlitePool> {
    let conn = Connection::open_in_memory().expect("in-memory database should always open");

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;",
    )
    .expect("PRAGMA setup should succeed");

    run_migrations(&conn, "0.1.0-test")
        .expect("migrations should apply cleanly to a fresh database");

    // Wrap the already-migrated connection using SqlitePool::from_connection.
    // This avoids re-opening a file (which would lose the in-memory schema)
    // while still going through the public SqlitePool API.
    Arc::new(SqlitePool::from_connection(conn))
}

// ---------------------------------------------------------------------------
// shell_preference_write_read_round_trip
// ---------------------------------------------------------------------------

/// Verify that saving a surface and loading it back returns the same value.
/// This is the core shell persistence invariant.
#[test]
fn shell_preference_write_read_round_trip() {
    let pool = migrated_pool();
    let store = ShellPreferenceStore::new(pool);

    store
        .save_active_surface(&Surface::History)
        .expect("save_active_surface should succeed");

    let loaded = store
        .load_active_surface()
        .expect("load_active_surface should succeed");

    assert_eq!(
        loaded,
        Some(Surface::History),
        "loaded surface should match the saved value"
    );
}

// ---------------------------------------------------------------------------
// shell_preference_startup_hydration_fresh_database
// ---------------------------------------------------------------------------

/// Verify that a fresh database (no row in shell_preferences) returns `None`
/// from `load_active_surface`, which the IPC handler treats as the default
/// Chat surface without falling back to frontend-local storage.
#[test]
fn shell_preference_startup_hydration_fresh_database_returns_none() {
    let pool = migrated_pool();
    let store = ShellPreferenceStore::new(pool);

    let loaded = store
        .load_active_surface()
        .expect("load on empty table should not error");

    assert_eq!(
        loaded, None,
        "fresh database should return None so the shell defaults to Chat"
    );
}

// ---------------------------------------------------------------------------
// shell_preference_overwrite_replaces_stored_value
// ---------------------------------------------------------------------------

/// Verify that a second save replaces the first — the UPSERT behavior is
/// correct and there is always at most one preference row.
#[test]
fn shell_preference_overwrite_replaces_stored_value() {
    let pool = migrated_pool();
    let store = ShellPreferenceStore::new(pool);

    store
        .save_active_surface(&Surface::Settings)
        .expect("first save should succeed");
    store
        .save_active_surface(&Surface::Artifacts)
        .expect("overwrite save should succeed");

    let loaded = store
        .load_active_surface()
        .expect("load after overwrite should succeed");

    assert_eq!(
        loaded,
        Some(Surface::Artifacts),
        "second save should replace the first stored value"
    );
}

// ---------------------------------------------------------------------------
// shell_preference_restore_non_default_surface_on_startup
// ---------------------------------------------------------------------------

/// Verify that a non-default surface stored in a previous session is restored
/// correctly — this is the restart-persistence contract that the plan requires.
///
/// The test simulates two sessions by using the same pool:
///   1. "Session 1" saves a non-default surface (History).
///   2. "Session 2" loads it back, confirming the value was not lost.
#[test]
fn shell_preference_restores_non_default_surface_on_startup() {
    let pool = migrated_pool();

    // Session 1: user navigated to History and the shell persisted it.
    {
        let store = ShellPreferenceStore::new(Arc::clone(&pool));
        store
            .save_active_surface(&Surface::History)
            .expect("session 1 save should succeed");
    }

    // Session 2: startup hydration loads the persisted preference.
    {
        let store = ShellPreferenceStore::new(Arc::clone(&pool));
        let loaded = store
            .load_active_surface()
            .expect("session 2 load should succeed");

        assert_eq!(
            loaded,
            Some(Surface::History),
            "restart should restore the non-default surface from session 1"
        );
    }
}

// ---------------------------------------------------------------------------
// shell_preference_all_surfaces_persist_correctly
// ---------------------------------------------------------------------------

/// Verify that every named surface can be round-tripped through persistence.
/// This guards against typos or missing FromStr/Display arms for any surface.
#[test]
fn shell_preference_all_surfaces_persist_correctly() {
    let surfaces = [
        Surface::Chat,
        Surface::History,
        Surface::Settings,
        Surface::Artifacts,
    ];

    for surface in surfaces {
        let pool = migrated_pool();
        let store = ShellPreferenceStore::new(pool);

        store
            .save_active_surface(&surface)
            .unwrap_or_else(|e| panic!("save {:?} failed: {e}", surface));

        let loaded = store
            .load_active_surface()
            .unwrap_or_else(|e| panic!("load {:?} failed: {e}", surface));

        assert_eq!(
            loaded,
            Some(surface.clone()),
            "surface {:?} did not survive persistence round trip",
            surface
        );
    }
}

// ---------------------------------------------------------------------------
// shell_surface_commands_keep_lock_order
// ---------------------------------------------------------------------------

/// Verify that `set_active_surface` can make progress while another thread
/// already holds the SQLite connection and that the old lock inversion does
/// not deadlock with `get_active_surface`.
///
/// The blocker thread holds the SQLite mutex, the setter takes the shell lock
/// first, and the getter then waits on the shell lock. If the old sqlite -> shell
/// order comes back, the timeout below trips instead of hanging forever.
#[test]
fn shell_surface_commands_keep_lock_order() {
    let pool = migrated_pool();
    let app_state = AppState::default();
    let store_state = ShellPreferenceStore::new(Arc::clone(&pool));

    let barrier = Arc::new(Barrier::new(3));
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (blocker_ready_tx, blocker_ready_rx) = mpsc::channel::<()>();
    let (setter_ready_tx, setter_ready_rx) = mpsc::channel::<()>();
    let (getter_ready_tx, getter_ready_rx) = mpsc::channel::<()>();
    let (setter_done_tx, setter_done_rx) = mpsc::channel::<Result<(), String>>();
    let (getter_done_tx, getter_done_rx) = mpsc::channel::<Result<Surface, String>>();

    thread::scope(|scope| {
        let barrier_for_blocker = Arc::clone(&barrier);
        let pool = Arc::clone(&pool);
        scope.spawn(move || {
            pool.with_conn(|_| {
                blocker_ready_tx.send(()).expect("blocker ready signal");
                barrier_for_blocker.wait();
                release_rx
                    .recv_timeout(Duration::from_secs(2))
                    .expect("release signal");
                Ok(())
            })
            .expect("blocker should keep the conn lock until released");
        });

        blocker_ready_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("blocker should acquire the conn lock");

        let shell_state_ref = &app_state;
        let store_state_ref = &store_state;
        let barrier_for_setter = Arc::clone(&barrier);
        scope.spawn(move || {
            setter_ready_tx.send(()).expect("setter ready signal");
            barrier_for_setter.wait();
            let result = tauri::async_runtime::block_on(set_active_surface_inner(
                shell_state_ref,
                store_state_ref,
                Surface::History,
            ))
            .map_err(|e| e.to_string());
            setter_done_tx
                .send(result)
                .expect("setter completion signal");
        });

        setter_ready_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("setter should start");

        barrier.wait();

        let shell_lock_held = std::time::Instant::now();
        while app_state.shell.try_lock().is_ok() {
            if shell_lock_held.elapsed() > Duration::from_secs(2) {
                panic!("setter never acquired the shell lock");
            }
            thread::yield_now();
        }

        let shell_state_ref = &app_state;
        let store_state_ref = &store_state;
        scope.spawn(move || {
            getter_ready_tx.send(()).expect("getter ready signal");
            let result = tauri::async_runtime::block_on(get_active_surface_inner(
                shell_state_ref,
                store_state_ref,
            ))
            .map_err(|e| e.to_string());
            getter_done_tx
                .send(result)
                .expect("getter completion signal");
        });

        getter_ready_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("getter should start");

        release_tx.send(()).expect("release blocker");

        assert_eq!(
            setter_done_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Ok(())
        );
        assert_eq!(
            getter_done_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Ok(Surface::History)
        );
    });

    let persisted = store_state
        .load_active_surface()
        .expect("load after command should succeed");

    assert_eq!(persisted, Some(Surface::History));
    assert_eq!(
        app_state
            .shell
            .lock()
            .expect("shell lock should not be poisoned")
            .active_surface,
        Surface::History
    );
}
