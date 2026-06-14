<script lang="ts">
	/**
	 * ChatSurface — live chat workspace surface.
	 *
	 * Wires `chatStore` to the three chat components:
	 * - `StreamingBubble` for the currently-streaming assistant message.
	 * - `ChatMessage` for all other messages.
	 * - `ChatInput` for the composition area with send/cancel.
	 *
	 * Auto-scroll: scrolls to bottom whenever the message list grows.
	 * Error display: shows the store error in an `aria-live` alert region.
	 */

	import { tick } from 'svelte';
	import { chatStore } from '$lib/stores/chat';
	import ChatInput from '$lib/components/chat/ChatInput.svelte';
	import ChatMessage from '$lib/components/chat/ChatMessage.svelte';
	import StreamingBubble from '$lib/components/chat/StreamingBubble.svelte';

	let messageListEl = $state<HTMLDivElement | undefined>(undefined);

	// Auto-scroll to the bottom when messages change.
	$effect(() => {
		// Reactive dependency on messages length.
		const _len = chatStore.messages.length;
		// Wait for the DOM update then scroll.
		tick().then(() => {
			if (messageListEl) {
				messageListEl.scrollTop = messageListEl.scrollHeight;
			}
		});
	});

	function handleSubmit(text: string): void {
		void chatStore.sendMessage(text);
	}
</script>

<div class="surface chat-surface" role="region" aria-label="Chat">
	<header class="surface-header">
		<h1 class="surface-title">Chat</h1>
	</header>

	<div class="surface-body">
		<!-- Message list -->
		<div
			class="message-list"
			bind:this={messageListEl}
			aria-live="polite"
			aria-label="Conversation"
			role="log"
		>
			{#each chatStore.messages as msg (msg.id)}
				{#if msg.id === chatStore.streamingId}
					<!-- Currently-streaming message: use StreamingBubble for hybrid UX (D-05) -->
					<StreamingBubble content={msg.content} loading={chatStore.loading} />
				{:else}
					<!-- Completed, incomplete, or errored message -->
					<ChatMessage
						role={msg.role}
						content={msg.content}
						status={msg.status}
						streaming={msg.streaming}
					/>
				{/if}
			{/each}
		</div>

		<!-- Error display -->
		{#if chatStore.error}
			<p class="chat-error" role="alert">{chatStore.error}</p>
		{/if}

		<!-- Input area with integrated cancel button (D-05) -->
		<ChatInput
			onsubmit={handleSubmit}
			disabled={chatStore.loading}
			showCancel={chatStore.canCancel}
			oncancel={chatStore.cancelRequest}
		/>
	</div>
</div>

<style>
	.surface {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: 100%;
	}

	.surface-header {
		padding: 16px 24px;
		border-bottom: 1px solid #2a2a2a;
		flex: 0 0 auto;
	}

	.surface-title {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
		color: #e0e0e0;
	}

	.surface-body {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.message-list {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		overflow-y: auto;
		padding: 12px 0;
		gap: 2px;
	}

	.chat-error {
		padding: 8px 16px;
		margin: 0;
		font-size: 13px;
		color: #e0a020;
		background: #2a2a1a;
		border-top: 1px solid #5a5a2a;
	}
</style>
