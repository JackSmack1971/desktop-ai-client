# Plan 011: Make shell hydration and surface persistence use the same lock order

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report -- do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 5f190a8..HEAD -- src-tauri/src/ipc/app_shell.rs src-tauri/tests/app_shell.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `5f190a8`, 2026-06-19

## Why this matters

`get_active_surface` currently takes the shell mutex and keeps it held while it
calls into SQLite. `set_active_surface` does the opposite: it writes to SQLite
first and only then takes the shell mutex to update in-memory state. That is a
classic lock inversion. If one thread is inside hydration while another is
inside a surface change, each can end up waiting on the lock the other already
holds. The result is a UI freeze instead of a clean surface update.

The fix should make both code paths use one order only. The least risky version
is to keep the shell mutex as the first lock in `set_active_surface` too, update
the in-memory state optimistically, persist to SQLite while still in that lock
order, and roll back the in-memory value if persistence fails. That removes the
cycle without changing the backend-owned persistence boundary.

## Current state

- `src-tauri/src/ipc/app_shell.rs` owns both commands. The relevant parts today:
  ```rust
  // get_active_surface
  let mut shell = state
      .shell
      .lock()
      .map_err(|e| ShellError::StorageError(format!("shell state lock poisoned: {e}")))?;

  if !shell.hydrated {
      if let Ok(Some(persisted)) = store.load_active_surface() {
          shell.active_surface = persisted;
      }
      shell.hydrated = true;
  }
  ```
  ```rust
  // set_active_surface
  store
      .save_active_surface(&surface)
      .map_err(|e| ShellError::StorageError(e.to_string()))?;

  let mut shell = state
      .shell
      .lock()
      .map_err(|e| ShellError::StorageError(format!("shell state lock poisoned: {e}")))?;
  shell.active_surface = surface;
  ```
- `src-tauri/src/storage/sqlite.rs` wraps the SQLite connection in a `Mutex<Connection>`:
  ```rust
  pub fn with_conn<F, T>(&self, f: F) -> rusqlite::Result<T>
  where
      F: FnOnce(&Connection) -> rusqlite::Result<T>,
  {
      let conn = self.conn.lock().unwrap_or_else(|poisoned| {
          panic!("SQLite connection mutex poisoned: {}", poisoned);
      });
      f(&conn)
  }
  ```
  Any caller that holds the shell mutex while waiting on `with_conn()` can
  deadlock against another caller doing the inverse.
- `src-tauri/tests/app_shell.rs` already has a storage-layer integration harness
  (`migrated_pool()`) you can reuse for the regression test. It currently only
  exercises round-trip persistence, not concurrency.

## Commands you will need

| Purpose        | Command                                                | Expected on success |
| -------------- | ------------------------------------------------------ | ------------------- |
| Compile check  | `cargo check --manifest-path src-tauri/Cargo.toml`     | exit 0              |
| Backend test   | `cargo test --manifest-path src-tauri/Cargo.toml --test app_shell` | all pass |

## Scope

**In scope** (the only files you should modify):

- `src-tauri/src/ipc/app_shell.rs`
- `src-tauri/tests/app_shell.rs`

**Out of scope** (do NOT touch, even though related):

- `src-tauri/src/storage/sqlite.rs` -- do not change the mutex wrapper or
  connection type. The bug is the lock order between the command handlers, not
  the storage primitive.
- `src-tauri/src/main.rs` and `src-tauri/tauri.conf.json` -- unrelated to the
  surface deadlock.
- Any frontend store or component -- the bug lives entirely in backend-owned
  surface persistence.

## Git workflow

- Branch: `advisor/011-unify-shell-lock-order`
- Commit message style: conventional commits, matching this repo's history
  (e.g. `fix(app-shell): remove shell/sqlite lock inversion`)
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Make `set_active_surface` follow the same lock order as hydration

In `src-tauri/src/ipc/app_shell.rs`, change `set_active_surface` so it takes the
shell mutex before it touches SQLite. Keep the existing crash-safe behavior as
much as possible by updating the in-memory surface optimistically, persisting
while still in the same lock order, and rolling the in-memory value back if
`save_active_surface()` fails.

The important shape is:

1. lock `state.shell`
2. save the previous in-memory value
3. update `shell.active_surface` to the requested surface
4. call `store.save_active_surface(&surface)` without releasing the shell lock
5. on error, restore the previous in-memory surface and return the storage error

Do not change `get_active_surface` into a different order unless you discover a
new constraint that makes this shape impossible. The goal is a single lock
order, not a separate refactor of the hydration flow.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` -> exit 0.

### Step 2: Add a regression test that exercises the concurrent shell/sqlite path

In `src-tauri/tests/app_shell.rs`, add a thread-based regression test that
proves the old cycle no longer deadlocks. Reuse the existing `migrated_pool()`
helper and `AppState::default()` as the fixture shape.

The test should:

1. start one thread that enters the surface persistence path
2. start a second thread that enters the hydration path
3. force the old interleaving with a `Barrier` or equivalent synchronization
4. assert both threads finish within a short timeout

Use `std::sync::Barrier` and `recv_timeout`/`join` with a timeout. Do not use
sleep-based timing; that makes the regression flaky instead of useful.

If you cannot get a reliable concurrency harness with the existing test shape,
stop and report rather than adding sleeps or weakening the assertion.

**Verify**: `cargo test --manifest-path src-tauri/Cargo.toml --test app_shell`
-> all tests pass, including the new deadlock regression.

## Test plan

- Add one concurrency regression test in `src-tauri/tests/app_shell.rs`.
- Keep the existing round-trip storage tests unchanged.
- The regression should fail on the old lock order and pass after the fix.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --test app_shell` exits 0
- [ ] `set_active_surface` and `get_active_surface` use one lock order only
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `set_active_surface` can no longer be made to take the shell lock before the
  SQLite write without breaking an explicit invariant you can point to in the
  live code.
- The regression test requires Tauri runtime scaffolding that the current
  `src-tauri/tests/app_shell.rs` harness does not already have and you cannot
  build a reliable timeout-based test with the existing public stores.
- A live drift check shows `get_active_surface` or `set_active_surface` has been
  refactored enough that this plan's current-state excerpts are no longer the
  right target.

## Maintenance notes

- If a future feature adds more than one shell surface writer, it should reuse
  the same lock order or move the in-memory state behind a single helper so the
  ordering cannot drift again.
- Reviewers should look specifically for any path that locks `state.shell`
  after touching SQLite. That is the failure mode this plan removes.
