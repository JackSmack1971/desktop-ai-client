/**
 * Reactive chat store using Svelte 5 runes.
 *
 * Manages the full conversation message list, streaming state, cancellation,
 * and error state. All streaming events arrive through the Tauri Channel and
 * are handled in `handleEvent` — no state changes happen outside that path
 * after a `chat_send` invocation (D-03: terminal state via channel only).
 *
 * Pattern: follows the `createSurfaceStore()` factory pattern from
 * `src/lib/stores/surface.ts` — factory function returning getter-only
 * reactive properties alongside async methods.
 */

import { chatSend, chatCancel } from '$lib/api/chat';
import type { ChatEvent } from '$lib/api/chat';

/** The persisted shape of a single chat message, including streaming state. */
export type ChatMessageState = {
	id: string;
	role: 'user' | 'assistant';
	content: string;
	/** True while the backend is still streaming tokens into this message. */
	streaming: boolean;
	/** `complete` = finished normally; `incomplete` = cancelled by user (D-06); `error` = terminal error. */
	status: 'complete' | 'incomplete' | 'error';
};

/**
 * Normalize an IPC rejection or ChatEvent Error to a user-facing string.
 *
 * Copied from `surface.ts` (not exported from there) — single source of truth
 * pattern: do not create a third copy; import from a shared util if a third
 * consumer appears.
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

function createChatStore() {
	/** Full ordered message list including user and assistant turns. */
	let messages = $state<ChatMessageState[]>([]);

	/** ID of the assistant message that is currently receiving streaming tokens. */
	let streamingId = $state<string | null>(null);

	/** Backend-assigned `request_id` from `ChatEvent::Ack`. Held for `chatCancel`. */
	let requestId = $state<string | null>(null);

	/**
	 * True from `sendMessage` call until the first `Delta` event arrives.
	 * Controls skeleton/thinking vs. streaming-bubble display (D-05).
	 */
	let loading = $state(false);

	/** User-facing error string, or null when no error is active. */
	let error = $state<string | null>(null);

	/** True when a cancellable request is in-flight. Derived from requestId. */
	let canCancel = $derived(requestId !== null);

	/**
	 * Handle a streaming event from the backend channel.
	 *
	 * `Ack`   — store request_id for cancel calls.
	 * `Delta` — first delta transitions placeholder to streaming bubble.
	 *           Subsequent deltas append to content.
	 * `Done`  — mark streaming message complete, clear stream state.
	 * `Error` (CANCELLED) — mark incomplete per D-06 (amber badge).
	 * `Error` (other)     — mark error, set error message.
	 */
	function handleEvent(event: ChatEvent): void {
		switch (event.type) {
			case 'Ack': {
				requestId = event.request_id;
				break;
			}
			case 'Delta': {
				// Transition from loading (thinking) to streaming on first delta (D-05).
				if (loading) {
					loading = false;
					// Activate the streaming bubble.
					messages = messages.map((m) =>
						m.id === streamingId ? { ...m, streaming: true } : m
					);
				}
				// Append delta text to the streaming message.
				messages = messages.map((m) =>
					m.id === streamingId ? { ...m, content: m.content + event.text } : m
				);
				break;
			}
			case 'Done': {
				// Mark the streaming message as complete.
				messages = messages.map((m) =>
					m.id === streamingId
						? { ...m, streaming: false, status: 'complete' }
						: m
				);
				streamingId = null;
				requestId = null;
				loading = false;
				break;
			}
			case 'Error': {
				if (event.code === 'CANCELLED') {
					// User-initiated cancel: freeze with amber badge (D-06).
					messages = messages.map((m) =>
						m.id === streamingId
							? { ...m, streaming: false, status: 'incomplete' }
							: m
					);
				} else {
					// Provider or channel error.
					error = normalizeIpcError(event);
					messages = messages.map((m) =>
						m.id === streamingId
							? { ...m, streaming: false, status: 'error' }
							: m
					);
				}
				streamingId = null;
				requestId = null;
				loading = false;
				break;
			}
		}
	}

	/**
	 * Submit a user message and begin streaming the assistant response.
	 *
	 * Appends the user message, inserts a placeholder assistant message,
	 * and invokes `chat_send`. Events arrive through `handleEvent`.
	 */
	async function sendMessage(content: string): Promise<void> {
		const userId = crypto.randomUUID();
		const assistantId = crypto.randomUUID();

		// Append user message.
		messages = [
			...messages,
			{ id: userId, role: 'user', content, streaming: false, status: 'complete' },
		];

		// Insert placeholder assistant message (thinking state per D-05).
		messages = [
			...messages,
			{
				id: assistantId,
				role: 'assistant',
				content: '',
				streaming: false, // not yet streaming — loading=true controls skeleton
				status: 'complete',
			},
		];

		streamingId = assistantId;
		loading = true;
		error = null;

		// Build the message list for the IPC call — only user/assistant turns,
		// no system prompt (backend-owned per D-12).
		const apiMessages = messages
			.filter((m) => m.id !== assistantId)
			.map((m) => ({ role: m.role, content: m.content })) as Array<{
			role: 'user' | 'assistant';
			content: string;
		}>;

		try {
			await chatSend({ messages: apiMessages, onEvent: handleEvent });
		} catch (e) {
			// invoke() rejection (e.g., CredentialError before stream starts).
			error = normalizeIpcError(e);
			// Remove the placeholder assistant message on invoke failure.
			messages = messages.filter((m) => m.id !== assistantId);
			streamingId = null;
			loading = false;
		}
	}

	/**
	 * Cancel the in-flight streaming request.
	 *
	 * The CANCELLED event will arrive through the channel and `handleEvent`
	 * will mark the message as `incomplete`. This is best-effort: if the
	 * request has already completed, the cancel is a no-op on the backend.
	 */
	async function cancelRequest(): Promise<void> {
		if (requestId === null) return;

		const idToCancel = requestId;
		try {
			await chatCancel(idToCancel);
		} catch (e) {
			// Cancel is best-effort. The channel will deliver the CANCELLED event
			// regardless, so log and ignore the rejection.
			console.warn('[chat-store] chatCancel rejected (best-effort):', e);
		}
	}

	return {
		get messages() {
			return messages;
		},
		get streamingId() {
			return streamingId;
		},
		get requestId() {
			return requestId;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		get canCancel() {
			return canCancel;
		},
		sendMessage,
		cancelRequest,
	};
}

/**
 * Singleton chat store exported for use across the chat surface.
 *
 * Usage:
 *   import { chatStore } from '$lib/stores/chat';
 *   chatStore.sendMessage('Hello!');
 */
export const chatStore = createChatStore();
