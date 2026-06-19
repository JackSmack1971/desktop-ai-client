# Phase 3: History - Pattern Map

**Mapped:** 2026-06-14
**Files analyzed:** 12
**Analogs found:** 10 / 12

## File Classification

| New/Modified File                                    | Role       | Data Flow                    | Closest Analog                                       | Match Quality |
| ---------------------------------------------------- | ---------- | ---------------------------- | ---------------------------------------------------- | ------------- |
| `src-tauri/src/ipc/history.rs`                       | controller | request-response (CRUD)      | `src-tauri/src/ipc/chat.rs`                          | role-match    |
| `src-tauri/src/storage/fts.rs`                       | service    | transform (query builder)    | `src-tauri/src/storage/sqlite.rs`                    | role-match    |
| `src-tauri/src/storage/retention.rs`                 | service    | CRUD                         | `src-tauri/src/storage/sqlite.rs`                    | role-match    |
| `src-tauri/src/storage/migrations.rs`                | config     | batch                        | `src-tauri/src/storage/migrations.rs` (self, append) | exact         |
| `src-tauri/src/main.rs`                              | config     | request-response             | `src-tauri/src/main.rs` (self, extend)               | exact         |
| `src-tauri/capabilities/main.json`                   | config     | —                            | `src-tauri/capabilities/main.json` (self, extend)    | exact         |
| `src/lib/components/surfaces/HistorySurface.svelte`  | component  | request-response             | `src/lib/components/surfaces/ChatSurface.svelte`     | exact         |
| `src/lib/components/history/SearchBar.svelte`        | component  | event-driven                 | `src/lib/components/chat/ChatInput.svelte`           | role-match    |
| `src/lib/components/history/ConversationList.svelte` | component  | request-response             | `src/lib/components/surfaces/ChatSurface.svelte`     | role-match    |
| `src/lib/components/history/ConversationRow.svelte`  | component  | request-response             | `src/lib/components/chat/ChatMessage.svelte`         | role-match    |
| `src/lib/stores/history.ts`                          | store      | request-response             | `src/lib/stores/surface.ts`                          | exact         |
| `src-tauri/src/ipc/chat.rs`                          | controller | request-response (streaming) | `src-tauri/src/ipc/chat.rs` (self, extend)           | exact         |

---

## Pattern Assignments

### `src-tauri/src/ipc/history.rs` (controller, request-response)

**Analog:** `src-tauri/src/ipc/chat.rs` and `src-tauri/src/ipc/app_shell.rs`

**Imports pattern** (from `app_shell.rs` lines 1-14, `chat.rs` lines 14-19):

```rust
use crate::app_state::AppState;
use crate::storage::sqlite::SqlitePool; // swapped for ConversationStore/MessageStore
```

**Error enum pattern** (`app_shell.rs` lines 18-27, `chat.rs` lines 69-82):

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HistoryError {
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized window: {0}")]
    UnauthorizedWindow(String),
}
```

Serializes to `{ "code": "STORAGE_ERROR", "message": "..." }` — same shape as `ShellError` and `ChatError`.

**Window guard pattern** (`app_shell.rs` lines 92-100, `chat.rs` lines 84-94):

```rust
fn assert_main_window(window: &tauri::Window) -> Result<(), HistoryError> {
    if window.label() != "main" {
        return Err(HistoryError::UnauthorizedWindow(format!(
            "history commands require the main window, got {:?}",
            window.label()
        )));
    }
    Ok(())
}
```

**Command skeleton pattern** (`app_shell.rs` lines 37-62):

```rust
#[tauri::command]
pub async fn history_list(
    window: tauri::Window,
    store: tauri::State<'_, ConversationStore>,
    // optional: limit: Option<u32>, cursor: Option<String>
) -> Result<Vec<ConversationSummary>, HistoryError> {
    assert_main_window(&window)?;
    store
        .list_conversations()
        .map_err(|e| HistoryError::StorageError(e.to_string()))
}
```

Repeat skeleton for `history_get`, `history_delete`, `history_search`. Each asserts the main window, delegates to the typed store, and maps `rusqlite::Error` to `HistoryError`.

**Test pattern** (`app_shell.rs` lines 103-113, `chat.rs` lines 326-421):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_error_serializes_with_code_field() {
        let err = HistoryError::StorageError("db locked".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("STORAGE_ERROR"), "expected SCREAMING_SNAKE_CASE: {json}");
    }
}
```

---

### `src-tauri/src/storage/fts.rs` (service, transform)

**Analog:** `src-tauri/src/storage/sqlite.rs`

**Typed store wrapper pattern** (`sqlite.rs` lines 76-82):

```rust
pub struct FtsStore {
    pool: std::sync::Arc<SqlitePool>,
}

impl FtsStore {
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }
    // expose: setup_fts_table(&Connection), search(query: &str) -> rusqlite::Result<Vec<SearchResult>>
}
```

**with_conn delegation pattern** (`sqlite.rs` lines 86-98):

```rust
pub fn search(&self, query: &str) -> rusqlite::Result<Vec<SearchResult>> {
    self.pool.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.title, c.model, c.status, c.updated_at,
                    snippet(messages_fts, 0, '<b>', '</b>', '…', 15) AS snippet
             FROM messages_fts
             JOIN conversations c ON messages_fts.conversation_id = c.id
             WHERE messages_fts MATCH ?1
             GROUP BY c.id
             ORDER BY rank
             LIMIT 50"
        )?;
        // map_err each row to rusqlite::Error
        Ok(vec![])
    })
}
```

**FTS5 setup SQL** (no analog — use RESEARCH.md / FTS5 docs):
The `setup_fts_table` method runs DDL inside `run_migrations`; do NOT call it from `FtsStore` directly. All DDL belongs in `migrations.rs` migration entries.

---

### `src-tauri/src/storage/retention.rs` (service, CRUD)

**Analog:** `src-tauri/src/storage/sqlite.rs` (`ShellPreferenceStore`)

**Typed store wrapper pattern** (`sqlite.rs` lines 76-82, 86-98):

```rust
pub struct RetentionStore {
    pool: std::sync::Arc<SqlitePool>,
}

impl RetentionStore {
    pub fn new(pool: std::sync::Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Hard-delete a conversation and all cascaded messages.
    /// Runs WAL checkpoint after delete; checkpoint errors are non-fatal.
    pub fn delete_conversation(&self, id: &str) -> rusqlite::Result<()> {
        self.pool.with_conn(|conn| {
            conn.execute(
                "DELETE FROM conversations WHERE id = ?1",
                rusqlite::params![id],
            )?;

            // WAL checkpoint — non-fatal if busy readers are present (D-14).
            let checkpoint_result = conn.execute_batch(
                "PRAGMA wal_checkpoint(TRUNCATE);"
            );
            if let Err(e) = checkpoint_result {
                // Log warning, do not propagate — delete already succeeded.
                eprintln!("[retention] WAL checkpoint warning after delete: {e}");
            }
            Ok(())
        })
    }
}
```

---

### `src-tauri/src/storage/migrations.rs` (config, batch — append only)

**Analog:** self (`src-tauri/src/storage/migrations.rs`)

**Migration entry pattern** (`migrations.rs` lines 33-45):

```rust
pub static MIGRATIONS: &[Migration] = &[
    Migration {
        id: "0001",
        description: "Create shell_preferences table for backend-owned surface state",
        sql: "
            CREATE TABLE IF NOT EXISTS shell_preferences (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              active_surface TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
        ",
    },
    // APPEND HERE — never reorder or modify existing entries.
    Migration {
        id: "0002",
        description: "Create conversations and messages tables",
        sql: "
            CREATE TABLE IF NOT EXISTS conversations (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL,
              model TEXT NOT NULL DEFAULT '',
              status TEXT NOT NULL DEFAULT 'active'
                CHECK (status IN ('active', 'complete', 'incomplete')),
              created_at TEXT NOT NULL DEFAULT (datetime('now')),
              updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE TABLE IF NOT EXISTS messages (
              id TEXT PRIMARY KEY,
              conversation_id TEXT NOT NULL
                REFERENCES conversations(id) ON DELETE CASCADE,
              role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
              content TEXT NOT NULL,
              status TEXT NOT NULL DEFAULT 'complete'
                CHECK (status IN ('complete', 'incomplete')),
              created_at TEXT NOT NULL DEFAULT (datetime('now'))
            ) STRICT;

            CREATE INDEX IF NOT EXISTS idx_messages_conversation_id
              ON messages(conversation_id);
        ",
    },
    Migration {
        id: "0003",
        description: "Create FTS5 external-content table and sync triggers for messages",
        sql: "
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
              content,
              conversation_id UNINDEXED,
              content='messages',
              content_rowid='rowid',
              tokenize='unicode61'
            );

            -- Sync triggers to keep messages_fts in step with messages rows.
            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
              INSERT INTO messages_fts(rowid, content, conversation_id)
                VALUES (new.rowid, new.content, new.conversation_id);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
              INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id)
                VALUES ('delete', old.rowid, old.content, old.conversation_id);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
              INSERT INTO messages_fts(messages_fts, rowid, content, conversation_id)
                VALUES ('delete', old.rowid, old.content, old.conversation_id);
              INSERT INTO messages_fts(rowid, content, conversation_id)
                VALUES (new.rowid, new.content, new.conversation_id);
            END;
        ",
    },
];
```

**Runner invariants** (`migrations.rs` lines 51-116):

- Each entry wrapped in `SAVEPOINT migration_{id} … RELEASE SAVEPOINT migration_{id}` by the runner — no need to add explicit transaction in the SQL string.
- After adding migrations, add a test asserting `MIGRATIONS.len()` equals the expected count.

---

### `src-tauri/src/main.rs` (config — extend existing)

**Analog:** self (`src-tauri/src/main.rs`)

**Registration pattern** (`main.rs` lines 13-14, 39-56):

```rust
// At top of main.rs — add new store imports:
use storage::sqlite::{ConversationStore, MessageStore, ShellPreferenceStore, SqlitePool};

// Inside .setup():
app.manage(ConversationStore::new(pool.clone()));
app.manage(MessageStore::new(pool.clone()));
// (RetentionStore and FtsStore can share the same pool arc)
app.manage(RetentionStore::new(pool.clone()));
app.manage(FtsStore::new(pool.clone()));

// Inside .invoke_handler():
.invoke_handler(tauri::generate_handler![
    ipc::app_shell::get_active_surface,
    ipc::app_shell::set_active_surface,
    ipc::chat::chat_send,
    ipc::chat::chat_cancel,
    ipc::history::history_list,    // new
    ipc::history::history_get,     // new
    ipc::history::history_delete,  // new
    ipc::history::history_search,  // new
])
```

---

### `src-tauri/capabilities/main.json` (config — extend existing)

**Analog:** self (`src-tauri/capabilities/main.json`)

**Permission entry pattern** (`main.json` lines 7-15):

```json
{
	"permissions": [
		"core:default",
		"opener:default",
		"core:app:allow-app-hide",
		"core:window:allow-start-dragging",
		"allow-get-active-surface",
		"allow-set-active-surface",
		"allow-chat-send",
		"allow-chat-cancel",
		"allow-history-list",
		"allow-history-get",
		"allow-history-delete",
		"allow-history-search"
	]
}
```

Each new `allow-history-*` entry requires a corresponding permission file under `src-tauri/permissions/` (created with `pnpm tauri permission new`).

---

### `src/lib/components/surfaces/HistorySurface.svelte` (component, request-response)

**Analog:** `src/lib/components/surfaces/ChatSurface.svelte`

**Surface layout pattern** (`ChatSurface.svelte` lines 1-130):

```svelte
<script lang="ts">
	import { historyStore } from '$lib/stores/history';
	import SearchBar from '$lib/components/history/SearchBar.svelte';
	import ConversationList from '$lib/components/history/ConversationList.svelte';
</script>

<div class="surface history-surface" role="region" aria-label="History">
	<header class="surface-header">
		<h1 class="surface-title">History</h1>
	</header>
	<div class="surface-body">
		<SearchBar onquery={(q) => void historyStore.search(q)} />
		{#if historyStore.error}
			<p class="history-error" role="alert">{historyStore.error}</p>
		{/if}
		<ConversationList
			conversations={historyStore.conversations}
			loading={historyStore.loading}
			ondelete={(id) => void historyStore.deleteConversation(id)}
		/>
	</div>
</div>
```

**Surface CSS pattern** (copy `.surface`, `.surface-header`, `.surface-title`, `.surface-body` blocks from `ChatSurface.svelte` lines 84-119 verbatim — identical grid/flex shell used by all surfaces).

---

### `src/lib/components/history/SearchBar.svelte` (component, event-driven)

**Analog:** `src/lib/components/chat/ChatInput.svelte`

**Props + $state pattern** (`ChatInput.svelte` lines 10-27):

```svelte
<script lang="ts">
	interface Props {
		onquery: (q: string) => void;
		disabled?: boolean;
	}
	let { onquery, disabled = false }: Props = $props();

	let text = $state('');
	let debounceTimer = $state<ReturnType<typeof setTimeout> | undefined>(
		undefined,
	);

	function handleInput(): void {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			onquery(text.trim());
		}, 300); // D-07: 300 ms debounce
	}
</script>
```

**Cleanup pattern** — return clearTimeout from `$effect` if the timer is managed reactively; otherwise clear in the input handler as above.

**Input element pattern** (`ChatInput.svelte` lines 38-47):

```svelte
<input
	type="search"
	class="search-input"
	bind:value={text}
	oninput={handleInput}
	{disabled}
	placeholder="Search conversations…"
	aria-label="Search conversation history"
/>
```

---

### `src/lib/components/history/ConversationList.svelte` (component, request-response)

**Analog:** `src/lib/components/surfaces/ChatSurface.svelte` (message list section, lines 44-67)

**Keyed each + loading pattern** (`ChatSurface.svelte` lines 52-67):

```svelte
<script lang="ts">
	import type { ConversationSummary } from '$lib/stores/history';
	import ConversationRow from '$lib/components/history/ConversationRow.svelte';

	interface Props {
		conversations: ConversationSummary[];
		loading: boolean;
		ondelete: (id: string) => void;
	}
	let { conversations, loading, ondelete }: Props = $props();
</script>

<div
	class="conversation-list"
	role="list"
	aria-label="Conversations"
	aria-busy={loading}
>
	{#each conversations as conv (conv.id)}
		<ConversationRow {conv} ondelete={() => ondelete(conv.id)} />
	{/each}
	{#if loading}
		<p class="list-loading" aria-live="polite">Loading…</p>
	{/if}
	{#if !loading && conversations.length === 0}
		<p class="list-empty">No conversations found.</p>
	{/if}
</div>
```

Stable key `conv.id` required (frontend rules: give every `{#each}` block a stable key for mutable lists).

---

### `src/lib/components/history/ConversationRow.svelte` (component, request-response)

**Analog:** `src/lib/components/chat/ChatMessage.svelte`

Read `ChatMessage.svelte` if needed for the exact prop/markup pattern; the key points are:

- `interface Props { conv: ConversationSummary; ondelete: () => void; }`
- `let { conv, ondelete }: Props = $props();`
- Display: `conv.title`, `conv.model` (badge), `conv.status` (chip), relative timestamp derived from `conv.updated_at`.
- Delete button calls `ondelete()` — no confirmation dialog in Phase 3.
- Row is a `<button>` or `<div role="button">` for keyboard navigation; click navigates to Chat surface with that conversation loaded.

---

### `src/lib/stores/history.ts` (store, request-response)

**Analog:** `src/lib/stores/surface.ts`

**Full store pattern** (`surface.ts` lines 1-115):

```typescript
import { invoke } from '@tauri-apps/api/core';

// Re-use the same normalizeIpcError helper — copy from surface.ts lines 32-40
// or import it if it becomes a shared utility.
function normalizeIpcError(e: unknown): string {
	if (typeof e === 'string') return e;
	if (e && typeof e === 'object') {
		const obj = e as Record<string, unknown>;
		if (typeof obj['message'] === 'string') return obj['message'];
		if (typeof obj['code'] === 'string') return `Error: ${obj['code']}`;
	}
	return 'An unexpected error occurred.';
}

export interface ConversationSummary {
	id: string;
	title: string;
	model: string;
	status: 'active' | 'complete' | 'incomplete';
	updatedAt: string; // ISO datetime string from Rust (camelCase per backend rule)
	snippet?: string; // only present for search results
}

function createHistoryStore() {
	let conversations = $state<ConversationSummary[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);

	// Pattern: async function, set loading true, call invoke, catch with normalizeIpcError
	async function load(): Promise<void> {
		loading = true;
		error = null;
		try {
			conversations = await invoke<ConversationSummary[]>('history_list');
		} catch (e) {
			error = normalizeIpcError(e);
		} finally {
			loading = false;
		}
	}

	async function search(query: string): Promise<void> {
		if (!query) {
			return load();
		}
		loading = true;
		error = null;
		try {
			conversations = await invoke<ConversationSummary[]>('history_search', {
				query,
			});
		} catch (e) {
			error = normalizeIpcError(e);
		} finally {
			loading = false;
		}
	}

	async function deleteConversation(id: string): Promise<void> {
		// Optimistic remove (same pattern as setSurface optimistic update in surface.ts lines 74-86)
		const previous = conversations;
		conversations = conversations.filter((c) => c.id !== id);
		try {
			await invoke<void>('history_delete', { id });
		} catch (e) {
			conversations = previous; // rollback
			error = normalizeIpcError(e);
		}
	}

	return {
		get conversations() {
			return conversations;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		load,
		search,
		deleteConversation,
	};
}

export const historyStore = createHistoryStore();
```

---

### `src-tauri/src/ipc/chat.rs` (controller — extend for storage writes)

**Analog:** self

**Extension point** (`chat.rs` lines 122-124):

```rust
// Phase 2 stub — conversation_id accepted but ignored:
let _ = &conversation_id;

// Phase 3 replacement: pass conversation_id into ConversationStore/MessageStore
// inside run_stream or at task completion when ChatEvent::Done fires.
// Store access goes through tauri::State<'_, ConversationStore> injected into chat_send.
// Lock ordering: do NOT hold shell lock while calling into ConversationStore (backend rules).
```

The conversation write must happen inside the spawned task (after `Done` event) because `tauri::State<'_>` is not `'static`. Use `app_handle.state::<ConversationStore>()` inside the spawned closure — same pattern as `app_handle.state::<AppState>()` on line 197.

---

## Shared Patterns

### Window-label enforcement

**Source:** `src-tauri/src/ipc/app_shell.rs` lines 92-100 and `src-tauri/src/ipc/chat.rs` lines 84-94
**Apply to:** `ipc/history.rs` (all four history commands)

```rust
fn assert_main_window(window: &tauri::Window) -> Result<(), HistoryError> {
    if window.label() != "main" {
        return Err(HistoryError::UnauthorizedWindow(format!(
            "history commands require the main window, got {:?}",
            window.label()
        )));
    }
    Ok(())
}
```

### Typed error enum (IPC shape)

**Source:** `src-tauri/src/ipc/app_shell.rs` lines 18-27
**Apply to:** `ipc/history.rs`

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HistoryError { ... }
```

Serializes to `{ "code": "SCREAMING_SNAKE_CASE", "message": "..." }`.

### `with_conn` delegation

**Source:** `src-tauri/src/storage/sqlite.rs` lines 56-68 (definition), lines 86-98 (use in `ShellPreferenceStore`)
**Apply to:** `ConversationStore`, `MessageStore`, `RetentionStore`, `FtsStore`

```rust
self.pool.with_conn(|conn| {
    // all DB work here; never hold this closure across an .await
    Ok(result)
})
```

Never call `with_conn` from IPC handlers directly — only from typed store methods.

### `normalizeIpcError` (frontend)

**Source:** `src/lib/stores/surface.ts` lines 32-40
**Apply to:** `src/lib/stores/history.ts`

```typescript
function normalizeIpcError(e: unknown): string {
	if (typeof e === 'string') return e;
	if (e && typeof e === 'object') {
		const obj = e as Record<string, unknown>;
		if (typeof obj['message'] === 'string') return obj['message'];
		if (typeof obj['code'] === 'string') return `Error: ${obj['code']}`;
	}
	return 'An unexpected error occurred.';
}
```

### Svelte 5 rune store factory

**Source:** `src/lib/stores/surface.ts` lines 42-114
**Apply to:** `src/lib/stores/history.ts`
Pattern: `function createXxxStore() { let x = $state(...); ... return { get x() { return x; }, ... }; } export const xxxStore = createXxxStore();`

### Surface CSS shell

**Source:** `src/lib/components/surfaces/ChatSurface.svelte` lines 84-119 and `src/lib/components/surfaces/HistorySurface.svelte` lines 21-55
**Apply to:** `HistorySurface.svelte` (already present in scaffold — extend, do not replace)
Classes: `.surface`, `.surface-header`, `.surface-title`, `.surface-body` — same flex-column, 100% height layout across all surfaces.

### Optimistic update with rollback

**Source:** `src/lib/stores/surface.ts` lines 74-87
**Apply to:** `historyStore.deleteConversation`

```typescript
const previous = conversations;
conversations = conversations.filter((c) => c.id !== id); // optimistic remove
try {
	await invoke<void>('history_delete', { id });
} catch (e) {
	conversations = previous; // rollback on failure
	error = normalizeIpcError(e);
}
```

### State injection via `app_handle.state<T>()` inside spawned tasks

**Source:** `src-tauri/src/ipc/chat.rs` lines 197-199
**Apply to:** `ipc/chat.rs` Phase 3 storage write (inside the spawned task)

```rust
let inner_state = app_handle.state::<ConversationStore>();
```

`tauri::State<'_>` is not `'static` — never move it into a `tokio::spawn` closure. Always re-acquire from `AppHandle` inside the task.

---

## No Analog Found

| File                                                   | Role    | Data Flow | Reason                                                                                        |
| ------------------------------------------------------ | ------- | --------- | --------------------------------------------------------------------------------------------- |
| `src-tauri/src/storage/fts.rs` (FTS5 DDL specifically) | service | transform | No FTS5 tables exist in codebase yet; DDL pattern comes from SQLite FTS5 docs and RESEARCH.md |

The `FtsStore` wrapper itself follows `ShellPreferenceStore` patterns exactly. Only the FTS5-specific SQL (virtual table DDL, `snippet()` auxiliary function, `MATCH` syntax) has no codebase precedent.

---

## Metadata

**Analog search scope:** `src-tauri/src/ipc/`, `src-tauri/src/storage/`, `src/lib/stores/`, `src/lib/components/`
**Files scanned:** 14
**Pattern extraction date:** 2026-06-14
