# Plan 006: Stop FTS5 syntax errors from surfacing as raw SQLite errors during conversation search

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- src-tauri/src/storage/fts.rs`
> If `fts.rs` changed since this plan was written, compare the "Current
> state" excerpt below against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

`FtsStore::search` binds the user's raw search text directly into a SQLite FTS5 `MATCH` expression. FTS5 treats the bound value as a _query expression_, not a plain literal — characters like unbalanced `"`, a leading `-`, or a bare boolean operator (`AND`/`OR`/`NOT`) have special syntactic meaning and cause SQLite to return a syntax error rather than "no results." `ipc::history::history_search` maps any such error straight into `HistoryError::StorageError(e.to_string())`, and the frontend's `normalizeIpcError` displays that message verbatim — so a user who types something as ordinary as `"unterminated` or a lone `-` into the search box sees a raw internal SQLite error message instead of an empty results list or a friendly "no matches."

## Current state

`src-tauri/src/storage/fts.rs:59-79` (`FtsStore::search`, relevant excerpt):

```rust
pub fn search(&self, query: &str) -> rusqlite::Result<Vec<SearchResult>> {
    self.pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.model, c.status, c.updated_at,
                    ( SELECT snippet(messages_fts, 0, '<b>', '</b>', '\u{2026}', 15)
                      FROM messages_fts
                      WHERE messages_fts.conversation_id = c.id AND messages_fts MATCH ?1
                      LIMIT 1 ) AS snippet
             FROM conversations c
             WHERE EXISTS (
                 SELECT 1 FROM messages_fts
                 WHERE messages_fts.conversation_id = c.id AND messages_fts MATCH ?1
             )
             ORDER BY c.updated_at DESC LIMIT 50",
        )?;
        let rows = stmt.query_map(rusqlite::params![query], |row| { /* ... */ })?;
        // ...
    })
}
```

`query` is bound as `?1` twice — both as a literal value (correctly avoiding SQL injection, per the file's own security-invariant doc comment at line 8-9) but its _content_ is still interpreted as FTS5 query syntax by SQLite's FTS5 extension, which is a different concern from SQL injection.

`src-tauri/src/ipc/history.rs:178-201` (`history_search`) calls `store.search(&query)` and maps `Err` to `HistoryError::StorageError(e.to_string())` — no special-casing of FTS5 syntax errors today.

## Commands you will need

| Purpose       | Command                                                              | Expected on success |
| ------------- | -------------------------------------------------------------------- | ------------------- |
| Compile check | `cargo check --manifest-path src-tauri/Cargo.toml`                   | exit 0              |
| Run tests     | `cargo test --manifest-path src-tauri/Cargo.toml --lib storage::fts` | all pass            |

## Scope

**In scope** (the only file you should modify):

- `src-tauri/src/storage/fts.rs`

**Out of scope** (do NOT touch, even though related):

- `src-tauri/src/ipc/history.rs` — no change needed there; once `FtsStore::search` itself treats arbitrary input as a safe literal phrase, no special error-mapping is needed at the IPC layer.
- The `snippet()` highlighting logic or the 50-row limit — unrelated, leave as-is.
- Any change to FTS5 ranking/relevance behavior — out of scope; this plan only changes how the raw query string is escaped before being treated as an FTS5 expression, not the ranking algorithm.

## Git workflow

- Branch: `advisor/006-quote-fts5-search-queries`
- Commit message: `fix(storage): treat FTS5 search input as a literal phrase, not a query expression`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Escape the query as an FTS5 quoted phrase before binding

FTS5's documented escaping rule for treating arbitrary text as a literal phrase (not a query expression) is: wrap the term in double quotes, and double any embedded double-quote characters. Add a small helper function above `search` in `fts.rs`:

```rust
/// Escape a raw user search string into a single FTS5 quoted phrase so that
/// special characters (`"`, `-`, `*`, boolean operators like AND/OR/NOT) are
/// treated as literal text rather than FTS5 query syntax. This trades away
/// FTS5's boolean/prefix query features in exchange for never throwing a
/// syntax error on ordinary user input — acceptable here since the search
/// box is a plain "find text in my conversations" field, not an advanced
/// query language exposed to the user.
fn escape_fts5_phrase(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}
```

Then change `search`'s body to escape the query once before using it in both bind sites:

```rust
pub fn search(&self, query: &str) -> rusqlite::Result<Vec<SearchResult>> {
    let escaped = escape_fts5_phrase(query);
    self.pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.model, c.status, c.updated_at,
                    ( SELECT snippet(messages_fts, 0, '<b>', '</b>', '\u{2026}', 15)
                      FROM messages_fts
                      WHERE messages_fts.conversation_id = c.id AND messages_fts MATCH ?1
                      LIMIT 1 ) AS snippet
             FROM conversations c
             WHERE EXISTS (
                 SELECT 1 FROM messages_fts
                 WHERE messages_fts.conversation_id = c.id AND messages_fts MATCH ?1
             )
             ORDER BY c.updated_at DESC LIMIT 50",
        )?;
        let rows = stmt.query_map(rusqlite::params![escaped], |row| { /* unchanged */ })?;
        // ...unchanged
    })
}
```

Only the value bound to `?1` changes (from `query` to `escaped`); the SQL text itself is unchanged.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0.

## Test plan

Add to `fts.rs`'s existing `#[cfg(test)] mod tests` block (which already has an in-memory pool fixture, `in_memory_pool()` — reuse it, do not write a new one):

```rust
#[test]
fn fts_search_does_not_error_on_unbalanced_quote() {
    let pool = in_memory_pool();
    let store = FtsStore::new(pool.clone());
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-q', 'Quote Test')", [],
        )?;
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-q', 'conv-q', 'user', 'some normal message text')", [],
        )?;
        Ok(())
    }).unwrap();

    // A lone unbalanced quote would be an FTS5 syntax error without escaping.
    let result = store.search("\"unterminated");
    assert!(result.is_ok(), "search with an unbalanced quote should not error: {result:?}");
}

#[test]
fn fts_search_treats_operator_keywords_as_literal_text() {
    let pool = in_memory_pool();
    let store = FtsStore::new(pool.clone());
    pool.with_conn(|conn| {
        conn.execute(
            "INSERT INTO conversations (id, title) VALUES ('conv-op', 'Operator Test')", [],
        )?;
        conn.execute(
            "INSERT INTO messages (id, conversation_id, role, content)
             VALUES ('msg-op', 'conv-op', 'user', 'find the AND keyword in this sentence')", [],
        )?;
        Ok(())
    }).unwrap();

    // "AND" would be parsed as a boolean operator (not literal text) without
    // escaping, and combined with no second operand would be a syntax error.
    let result = store.search("AND");
    assert!(result.is_ok(), "search for a bare boolean-operator keyword should not error: {result:?}");
}
```

Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib storage::fts` → all pass, including the existing `fts_search_returns_result_with_snippet` and `fts_search_returns_empty_vec_on_no_match` tests (unaffected) plus the 2 new ones.

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib storage::fts` exits 0, all 4 tests (2 existing + 2 new) pass
- [ ] `FtsStore::search` calls `escape_fts5_phrase` before binding the query
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- `FtsStore::search`'s SQL text or bind-parameter structure no longer matches the excerpt above.
- The new tests reveal that escaping breaks the _existing_ passing test `fts_search_returns_result_with_snippet` (i.e., normal single-word searches stop matching) — this would mean the escaping function has a bug; do not ship a fix that breaks existing search behavior.

## Maintenance notes

- This intentionally gives up FTS5's boolean-query and prefix-wildcard features (`term*`, `term1 OR term2`) in exchange for never erroring on ordinary text. If the product later wants an "advanced search" mode with real FTS5 operators, that would need a separate, explicitly-opted-into code path — not a reason to revert this fix for the default search box.
