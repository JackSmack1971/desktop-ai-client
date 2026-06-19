/**
 * Typed TypeScript wrapper over Tauri IPC for the chat streaming surface.
 *
 * This is the only file in the frontend that imports from `@tauri-apps/api/core`.
 * All other frontend modules import `chatSend`, `chatCancel`, and the `ChatEvent`
 * type from here. This keeps the Tauri API surface isolated and mockable.
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
 * produces the `type` discriminator used here.
 */
export type ChatEvent =
	| { type: 'Ack'; request_id: string }
	| { type: 'Delta'; text: string }
	| { type: 'Done'; usage?: { prompt_tokens: number; completion_tokens: number }; model: string }
	| { type: 'Error'; code: string; message: string }
	| {
			type: 'ArtifactReady';
			conversation_id: string;
			artifact_id: string;
			content_type: ArtifactContentType;
			preview: string;
	  };

/** A message in the conversation history. Role is constrained to the two valid values. */
export type ChatMessage = { role: 'user' | 'assistant'; content: string };

/** Parameters for `chatSend`. The `onEvent` callback receives each streaming event. */
export type ChatSendParams = {
	messages: ChatMessage[];
	model?: string;
	conversationId?: string;
	maxCompletionTokens?: number;
	temperature?: number;
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
		messages: params.messages,
		model: params.model ?? null,
		conversationId: params.conversationId ?? null,
		maxCompletionTokens: params.maxCompletionTokens ?? null,
		temperature: params.temperature ?? null,
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
export async function chatCancel(requestId: string): Promise<void> {
	await invoke('chat_cancel', { requestId });
}
