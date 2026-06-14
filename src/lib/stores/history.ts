/**
 * Backend-owned conversation history store.
 *
 * All reads and writes go through typed IPC commands. This store never reads
 * localStorage, sessionStorage, or any browser storage. Conversation content
 * stays backend-owned and crosses IPC only through the typed history_* commands.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Summary of a single conversation, returned by history_list and history_search.
 * Fields map 1-to-1 to the Rust ConversationSummary struct serialized over IPC.
 */
export interface ConversationSummary {
	id: string;
	title: string;
	model: string;
	status: 'active' | 'complete' | 'incomplete';
	updatedAt: string; // ISO datetime string from Rust (camelCase per backend rule)
	snippet?: string;  // only present for search results (FTS5 snippet() output)
}

/**
 * Normalize an IPC rejection to a user-facing error string.
 *
 * Tauri rejects IPC calls with serialized HistoryError objects
 * ({ code, message }) or plain strings. String(e) on an object
 * produces "[object Object]", which is useless in the UI.
 */
function normalizeIpcError(e: unknown): string {
	if (typeof e === 'string') return e;
	if (e && typeof e === 'object') {
		const obj = e as Record<string, unknown>;
		if (typeof obj['message'] === 'string') return obj['message'];
		if (typeof obj['code'] === 'string') return `Error: ${obj['code']}`;
	}
	return 'An unexpected error occurred.';
}

function createHistoryStore() {
	let conversations = $state<ConversationSummary[]>([]);
	let loading = $state(false);
	let error = $state<string | null>(null);
	let activeConversationId = $state<string | null>(null);

	/**
	 * Load all conversations from backend-owned SQLite storage.
	 * Called once on HistorySurface mount and after search query is cleared.
	 */
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

	/**
	 * Search conversations by message content using FTS5.
	 * If query is empty or whitespace-only, falls back to load() to restore full list.
	 * The 300ms debounce is handled in the SearchBar component (D-07).
	 */
	async function search(query: string): Promise<void> {
		if (!query.trim()) { return load(); }
		loading = true;
		error = null;
		try {
			conversations = await invoke<ConversationSummary[]>('history_search', { query: query.trim() });
		} catch (e) {
			error = normalizeIpcError(e);
		} finally {
			loading = false;
		}
	}

	/**
	 * Delete a conversation by ID with optimistic removal.
	 * Removes the row immediately for responsive UI, then calls the backend.
	 * On IPC failure, restores the previous list and sets error.
	 */
	async function deleteConversation(id: string): Promise<void> {
		const previous = conversations;
		conversations = conversations.filter(c => c.id !== id); // optimistic remove
		try {
			await invoke<void>('history_delete', { id });
		} catch (e) {
			conversations = previous; // rollback on failure
			error = normalizeIpcError(e);
		}
	}

	/**
	 * Set the active conversation ID.
	 * The HistorySurface component calls surfaceStore.setSurface('chat') after this
	 * to complete the D-10 navigation flow.
	 */
	function loadConversation(id: string): void {
		activeConversationId = id;
	}

	return {
		get conversations() { return conversations; },
		get loading() { return loading; },
		get error() { return error; },
		get activeConversationId() { return activeConversationId; },
		load,
		search,
		deleteConversation,
		loadConversation,
	};
}

/**
 * Singleton history store exported for use across the shell.
 *
 * Usage:
 *   import { historyStore } from '$lib/stores/history';
 *   void historyStore.load(); // on mount
 */
export const historyStore = createHistoryStore();
