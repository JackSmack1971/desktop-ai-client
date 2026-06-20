/**
 * Typed TypeScript wrapper over Tauri IPC for the chat streaming surface.
 *
 * This module imports from `@tauri-apps/api/core` and keeps the chat IPC surface
 * isolated and mockable. Other frontend modules should import `chatSend`,
 * `chatCancel`, and the `ChatEvent` type from here instead of reaching for Tauri
 * APIs directly.
 *
 * Security: this module never constructs or passes an `api_key` value. Credentials
 * stay backend-owned per D-10. The frontend only sends conversation content.
 */

import { Channel, invoke } from '@tauri-apps/api/core';

export type ArtifactContentType =
	| { type: 'Html' }
	| { type: 'Svg' }
	| { type: 'PlainText' }
	| { type: 'Code'; language: string };

/**
 * Streaming event variants delivered through `Channel<ChatEvent>` from Rust.
 *
 * Must match `ChatEvent` enum in `src-tauri/src/ipc/chat.rs` exactly.
 * The Rust `#[serde(tag = "type", rename_all = "PascalCase")]` attribute
 * produces the `type` discriminator used here. Every variant carries a
 * `sequence` — strictly increasing per attempt, starting at `Ack` — so the
 * frontend can detect dropped/out-of-order delivery if it ever matters.
 */
export type ChatEvent =
	| {
			type: 'Ack';
			conversation_id: string;
			turn_id: string;
			attempt_id: string;
			attempt_number: number;
			sequence: number;
	  }
	| { type: 'Delta'; text: string; sequence: number }
	| {
			type: 'Done';
			usage?: { prompt_tokens: number; completion_tokens: number };
			model: string;
			sequence: number;
	  }
	| { type: 'Error'; code: string; message: string; sequence: number }
	| {
			type: 'ArtifactReady';
			conversation_id: string;
			artifact_id: string;
			content_type: ArtifactContentType;
			preview: string;
			sequence: number;
	  };

/** A message in the conversation history. Role is constrained to the two valid values. */
export type ChatMessage = { role: 'user' | 'assistant'; content: string };

/**
 * Parameters for `chatSend`. The `onEvent` callback receives each streaming event.
 *
 * `history` is prior conversation context for the provider only — every
 * message in it is already persisted and must NOT be sent again as part of
 * `newMessage`. `idempotencyKey` must be stable across retries of the same
 * turn: a brand-new message gets a freshly generated key; retrying a
 * failed/cancelled turn reuses the key from that turn so the backend never
 * inserts a duplicate user message.
 */
export type ChatSendParams = {
	history: ChatMessage[];
	newMessage: ChatMessage;
	idempotencyKey: string;
	model?: string;
	conversationId?: string;
	maxCompletionTokens?: number;
	temperature?: number;
	/**
	 * Hint for the backend Policy-Constrained Provider Runtime. `'strict'`
	 * requests zero-data-retention-eligible routing; the backend resolves
	 * this against its reviewed model allowlist and fails the request rather
	 * than silently downgrading when it can't be satisfied. Omit for the
	 * default `'standard'` tier.
	 */
	privacyMode?: 'standard' | 'strict';
	attachments?: string[];
	onEvent: (event: ChatEvent) => void;
};

/**
 * Submit a prompt and receive streamed response events via `onEvent`.
 *
 * Creates a typed Channel, registers the event handler, then invokes `chat_send`.
 * Tauri automatically converts camelCase JS param keys to snake_case Rust param names.
 *
 * The `invoke` call returns once the Rust command returns `Ok(())`, which happens
 * immediately after the spawned task starts — not after streaming completes.
 * Events arrive asynchronously through `onEvent` until `Done` or `Error`.
 */
export async function chatSend(params: ChatSendParams): Promise<void> {
	const channel = new Channel<ChatEvent>();
	channel.onmessage = params.onEvent;

	await invoke('chat_send', {
		history: params.history,
		newMessage: params.newMessage,
		idempotencyKey: params.idempotencyKey,
		model: params.model ?? null,
		conversationId: params.conversationId ?? null,
		maxCompletionTokens: params.maxCompletionTokens ?? null,
		temperature: params.temperature ?? null,
		privacyMode: params.privacyMode ?? null,
		attachments: params.attachments ?? null,
		channel,
	});
}

/**
 * Cancel an in-flight streaming request.
 *
 * The backend will signal the CancellationToken and emit `ChatEvent::Error`
 * with `code === 'CANCELLED'` through the channel to close the listener.
 * Cancellation is best-effort — the event may arrive slightly after calling this.
 */
export async function chatCancel(attemptId: string): Promise<void> {
	await invoke('chat_cancel', { attemptId });
}
