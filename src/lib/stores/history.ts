/**
 * Backend-owned conversation history store.
 *
 * All reads and writes go through typed IPC commands. This store never reads
 * localStorage, sessionStorage, or any browser storage. Conversation content
 * stays backend-owned and crosses IPC only through the typed history_* commands.
 */

import { invoke } from '@tauri-apps/api/core';
import { normalizeIpcError } from '$lib/api/errors';

/**
 * Summary of a single conversation, returned by history_list and history_search.
 * Fields map 1-to-1 to the Rust ConversationSummary struct serialized over IPC
 * (`#[serde(rename_all = "camelCase")]` in `ipc::history`).
 */
export interface ConversationSummary {
	id: string;
	title: string;
	model: string;
	status: 'active' | 'complete' | 'incomplete';
	updatedAt: string; // ISO datetime string from Rust (camelCase per backend rule)
	snippet?: string; // only present for search results (FTS5 snippet() output)
}

/** A single persisted message, returned as part of `ConversationDetail`. */
export interface MessageSummary {
	id: string;
	role: 'user' | 'assistant';
	content: string;
	status: 'complete' | 'incomplete';
	createdAt: string;
}

/** Full conversation record with its message list, returned by `history_get`. */
export interface ConversationDetail {
	id: string;
	title: string;
	model: string;
	status: 'active' | 'complete' | 'incomplete';
	updatedAt: string;
	messages: MessageSummary[];
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
		if (!query.trim()) {
			return load();
		}
		loading = true;
		error = null;
		try {
			conversations = await invoke<ConversationSummary[]>('history_search', {
				query: query.trim(),
			});
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
		conversations = conversations.filter((c) => c.id !== id); // optimistic remove
		try {
			await invoke<void>('history_delete', { id });
		} catch (e) {
			conversations = previous; // rollback on failure
			error = normalizeIpcError(e);
		}
	}

	/**
	 * Fetch a conversation's full message list and mark it active.
	 *
	 * The caller (HistorySurface) is responsible for hydrating `chatStore`
	 * with the returned detail's `messages` and then calling
	 * `surfaceStore.setSurface('chat')` to complete the D-10 navigation flow
	 * — this store does not depend on `chatStore` to avoid a circular import.
	 * Returns `null` on failure; `error` is set in that case.
	 */
	async function loadConversation(id: string): Promise<ConversationDetail | null> {
		error = null;
		try {
			const detail = await invoke<ConversationDetail>('history_get', { id });
			activeConversationId = id;
			return detail;
		} catch (e) {
			error = normalizeIpcError(e);
			return null;
		}
	}

	/**
	 * Stabilize the active conversation id once the backend assigns one.
	 *
	 * Called by `chatStore` after the first `chat_send` Ack for a brand-new
	 * conversation, so every later send in this session reuses the same
	 * `conversation_id` instead of creating a new conversation each time.
	 */
	function setActiveConversationId(id: string): void {
		activeConversationId = id;
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
		get activeConversationId() {
			return activeConversationId;
		},
		load,
		search,
		deleteConversation,
		loadConversation,
		setActiveConversationId,
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
