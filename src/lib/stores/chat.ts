/**
 * Reactive chat store using Svelte 5 runes.
 *
 * Manages the full conversation message list, streaming state, cancellation,
 * and error state. All streaming events arrive through the Tauri Channel and
 * are handled in `handleEvent` — no state changes happen outside that path
 * after a `chat_send` invocation (D-03: terminal state via channel only).
 *
 * Conversation Transaction Protocol:
 * - `conversation_id` is learned from the backend's `Ack` event the first
 *   time a message is sent without one, and stabilized into `historyStore`
 *   so every later send in this session reuses it (see `chat_send`'s
 *   conversation_id contract in `src-tauri/src/ipc/chat.rs`).
 * - Each send/retry carries a client-generated `idempotencyKey`. A brand-new
 *   message gets a fresh key; `retryMessage` reuses the key from the failed
 *   turn so the backend never inserts a duplicate user message.
 * - `hydrate()` replaces the message list with a conversation already
 *   persisted on the backend (used when switching conversations from
 *   History) — it never re-sends those messages to the backend.
 *
 * Known limitation: `idempotencyKey` is only tracked in memory for messages
 * created in this session. A message loaded via `hydrate()` from history has
 * no known idempotencyKey, so `retryMessage` is a no-op for it — retrying an
 * old incomplete turn after reopening it from History is not yet supported.
 *
 * Pattern: follows the `createSurfaceStore()` factory pattern from
 * `src/lib/stores/surface.ts` — factory function returning getter-only
 * reactive properties alongside async methods.
 */

import { chatSend, chatCancel } from '$lib/api/chat';
import type { ChatEvent, ChatMessage } from '$lib/api/chat';
import { normalizeIpcError } from '$lib/api/errors';
import { artifactsStore } from '$lib/stores/artifacts';
import { historyStore } from '$lib/stores/history';
import type { MessageSummary } from '$lib/stores/history';

/** The persisted shape of a single chat message, including streaming state. */
export type ChatMessageState = {
	id: string;
	role: 'user' | 'assistant';
	content: string;
	/** True while the backend is still streaming tokens into this message. */
	streaming: boolean;
	/** `complete` = finished normally; `incomplete` = cancelled or partial (D-06); `error` = terminal error. */
	status: 'complete' | 'incomplete' | 'error';
	/**
	 * Set on both messages of a turn created in this session. Lets
	 * `retryMessage` resubmit the same turn under the same idempotency key
	 * instead of creating a duplicate. Absent on messages loaded via
	 * `hydrate()`.
	 */
	idempotencyKey?: string;
};

function createChatStore() {
	/** Full ordered message list including user and assistant turns. */
	let messages = $state<ChatMessageState[]>([]);

	/** ID of the assistant message that is currently receiving streaming tokens. */
	let streamingId = $state<string | null>(null);

	/** Backend-assigned `attempt_id` from `ChatEvent::Ack`. Held for `chatCancel`. */
	let attemptId = $state<string | null>(null);

	/**
	 * True from `sendMessage`/`retryMessage` until the first `Delta` event arrives.
	 * Controls skeleton/thinking vs. streaming-bubble display (D-05).
	 */
	let loading = $state(false);

	/** User-facing error string, or null when no error is active. */
	let error = $state<string | null>(null);

	/** True when a cancellable request is in-flight. Derived from attemptId. */
	let canCancel = $derived(attemptId !== null);

	/**
	 * Handle a streaming event from the backend channel.
	 *
	 * `Ack`   — store attempt_id for cancel calls; stabilize conversation_id
	 *           into historyStore the first time a new conversation is created.
	 * `Delta` — first delta transitions placeholder to streaming bubble.
	 *           Subsequent deltas append to content.
	 * `Done`  — mark streaming message complete, clear stream state.
	 * `Error` (CANCELLED / FAILED_PARTIAL) — mark incomplete; partial text is
	 *          already in `content` from prior Delta events (D-06).
	 * `Error` (other)     — mark error, set error message.
	 */
	function handleEvent(event: ChatEvent): void {
		switch (event.type) {
			case 'Ack': {
				attemptId = event.attempt_id;
				if (historyStore.activeConversationId === null) {
					historyStore.setActiveConversationId(event.conversation_id);
				}
				break;
			}
			case 'Delta': {
				// Transition from loading (thinking) to streaming on first delta (D-05).
				if (loading) {
					loading = false;
					// Activate the streaming bubble.
					messages = messages.map((m) =>
						m.id === streamingId ? { ...m, streaming: true } : m,
					);
				}
				// Append delta text to the streaming message.
				messages = messages.map((m) =>
					m.id === streamingId ? { ...m, content: m.content + event.text } : m,
				);
				break;
			}
			case 'Done': {
				// Mark the streaming message as complete.
				messages = messages.map((m) =>
					m.id === streamingId
						? { ...m, streaming: false, status: 'complete' }
						: m,
				);
				streamingId = null;
				attemptId = null;
				loading = false;
				break;
			}
			case 'Error': {
				if (event.code === 'CANCELLED' || event.code === 'FAILED_PARTIAL') {
					// User-initiated cancel or a truncated stream: freeze with
					// amber badge, preserving whatever partial text streamed (D-06).
					messages = messages.map((m) =>
						m.id === streamingId
							? { ...m, streaming: false, status: 'incomplete' }
							: m,
					);
				} else {
					// Provider, storage, or channel error.
					error = normalizeIpcError(event);
					messages = messages.map((m) =>
						m.id === streamingId
							? { ...m, streaming: false, status: 'error' }
							: m,
					);
				}
				streamingId = null;
				attemptId = null;
				loading = false;
				break;
			}
			case 'ArtifactReady': {
				artifactsStore.receiveArtifact(event, historyStore.activeConversationId);
				break;
			}
		}
	}

	/** Build the `{history, newMessage}` IPC payload, excluding the system prompt (D-12). */
	function buildHistoryAndNewMessage(
		userId: string,
		assistantId: string,
		userContent: string,
	): { history: ChatMessage[]; newMessage: ChatMessage } {
		const history = messages
			.filter((m) => m.id !== userId && m.id !== assistantId)
			.map((m) => ({ role: m.role, content: m.content }) satisfies ChatMessage);
		return { history, newMessage: { role: 'user', content: userContent } };
	}

	/** Shared submission tail for both a brand-new send and a retry. */
	async function submitTurn(
		userId: string,
		assistantId: string,
		idempotencyKey: string,
		userContent: string,
		removeOnInvokeFailure: boolean,
	): Promise<void> {
		streamingId = assistantId;
		loading = true;
		error = null;

		const { history, newMessage } = buildHistoryAndNewMessage(userId, assistantId, userContent);

		try {
			await chatSend({
				history,
				newMessage,
				idempotencyKey,
				conversationId: historyStore.activeConversationId ?? undefined,
				onEvent: handleEvent,
			});
		} catch (e) {
			// invoke() rejection (e.g., CredentialError, DuplicateInFlight before
			// any Ack — nothing was persisted for this attempt).
			error = normalizeIpcError(e);
			if (removeOnInvokeFailure) {
				messages = messages.filter((m) => m.id !== userId && m.id !== assistantId);
			} else {
				messages = messages.map((m) =>
					m.id === assistantId ? { ...m, status: 'error', streaming: false } : m,
				);
			}
			streamingId = null;
			loading = false;
		}
	}

	/**
	 * Submit a user message and begin streaming the assistant response.
	 *
	 * Appends the user message, inserts a placeholder assistant message, and
	 * invokes `chat_send` with a freshly generated idempotency key. Events
	 * arrive through `handleEvent`.
	 */
	async function sendMessage(content: string): Promise<void> {
		const userId = crypto.randomUUID();
		const assistantId = crypto.randomUUID();
		const idempotencyKey = crypto.randomUUID();

		messages = [
			...messages,
			{
				id: userId,
				role: 'user',
				content,
				streaming: false,
				status: 'complete',
				idempotencyKey,
			},
			{
				id: assistantId,
				role: 'assistant',
				content: '',
				streaming: false, // not yet streaming — loading=true controls skeleton
				status: 'complete',
				idempotencyKey,
			},
		];

		await submitTurn(userId, assistantId, idempotencyKey, content, true);
	}

	/**
	 * Retry the turn that produced `assistantMessageId`, reusing its original
	 * idempotency key so the backend resumes the same turn (new attempt)
	 * instead of inserting a duplicate user message.
	 *
	 * No-op if the message has no known idempotency key (e.g. it was loaded
	 * via `hydrate()` rather than created in this session) or isn't preceded
	 * by its user message.
	 */
	async function retryMessage(assistantMessageId: string): Promise<void> {
		const idx = messages.findIndex((m) => m.id === assistantMessageId);
		if (idx <= 0) return;
		const assistantMsg = messages[idx];
		const userMsg = messages[idx - 1];
		if (!assistantMsg.idempotencyKey || userMsg.role !== 'user') return;

		messages = messages.map((m) =>
			m.id === assistantMessageId
				? { ...m, content: '', status: 'complete', streaming: false }
				: m,
		);

		await submitTurn(userMsg.id, assistantMessageId, assistantMsg.idempotencyKey, userMsg.content, false);
	}

	/**
	 * Replace the message list with a conversation already persisted on the
	 * backend. Called when switching conversations from History — these
	 * messages are NOT re-sent; only a new turn appended after hydration is.
	 */
	function hydrate(historyMessages: MessageSummary[]): void {
		messages = historyMessages.map((m) => ({
			id: m.id,
			role: m.role === 'assistant' ? 'assistant' : 'user',
			content: m.content,
			streaming: false,
			status: m.status === 'incomplete' ? 'incomplete' : 'complete',
		}));
		streamingId = null;
		attemptId = null;
		loading = false;
		error = null;
	}

	/** Clear the conversation back to a blank slate (e.g. "New chat"). */
	function reset(): void {
		messages = [];
		streamingId = null;
		attemptId = null;
		loading = false;
		error = null;
	}

	/**
	 * Cancel the in-flight streaming request.
	 *
	 * The CANCELLED event will arrive through the channel and `handleEvent`
	 * will mark the message as `incomplete`. This is best-effort: if the
	 * request has already completed, the cancel is a no-op on the backend.
	 */
	async function cancelRequest(): Promise<void> {
		if (attemptId === null) return;

		const idToCancel = attemptId;
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
		get attemptId() {
			return attemptId;
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
		retryMessage,
		hydrate,
		reset,
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
