<script lang="ts">
	/**
	 * ChatMessage — renders a single conversation bubble.
	 *
	 * Handles three visual states:
	 * - `streaming === true`: shows blinking cursor after content.
	 * - `status === 'incomplete'`: dimmed content + amber "(Cancelled)" badge (D-06).
	 * - `status === 'error'`: error styling on content.
	 *
	 * An assistant message in `incomplete` or `error` status shows a Retry
	 * action, calling `onRetry(id)` — wired to `chatStore.retryMessage` by
	 * `ChatSurface`. Resubmits the turn under its original idempotency key
	 * rather than creating a duplicate (see `chatStore.retryMessage`).
	 *
	 * User messages: right-aligned, accent background.
	 * Assistant messages: left-aligned, card background.
	 */

	interface Props {
		id: string;
		role: 'user' | 'assistant';
		content: string;
		status: 'complete' | 'incomplete' | 'error';
		streaming: boolean;
		onRetry?: (id: string) => void;
	}

	let { id, role, content, status, streaming, onRetry }: Props = $props();

	let isUser = $derived(role === 'user');
	let isIncomplete = $derived(status === 'incomplete');
	let isError = $derived(status === 'error');
	let canRetry = $derived(!isUser && (isIncomplete || isError) && onRetry !== undefined);
</script>

<div class="message-wrapper" class:user={isUser} class:assistant={!isUser}>
	<div
		class="message-bubble"
		class:user-bubble={isUser}
		class:assistant-bubble={!isUser}
		class:error-bubble={isError}
	>
		<span class="message-content" class:dimmed={isIncomplete}>{content}</span>

		{#if streaming}
			<span class="cursor" aria-hidden="true">|</span>
		{/if}

		{#if isIncomplete}
			<span
				class="cancelled-badge"
				role="status"
				aria-label="Response was interrupted">(Interrupted)</span
			>
		{/if}

		{#if canRetry}
			<button type="button" class="retry-button" onclick={() => onRetry?.(id)}>
				Retry
			</button>
		{/if}
	</div>
</div>

<style>
	.message-wrapper {
		display: flex;
		margin: 4px 0;
		padding: 0 16px;
	}

	.message-wrapper.user {
		justify-content: flex-end;
	}

	.message-wrapper.assistant {
		justify-content: flex-start;
	}

	.message-bubble {
		max-width: 75%;
		padding: 10px 14px;
		border-radius: 12px;
		font-size: 14px;
		line-height: 1.6;
		white-space: pre-wrap;
		word-break: break-word;
	}

	.user-bubble {
		background: #1e4a7a;
		color: #e0e0e0;
		border-bottom-right-radius: 3px;
	}

	.assistant-bubble {
		background: #242424;
		color: #e0e0e0;
		border: 1px solid #2a2a2a;
		border-bottom-left-radius: 3px;
	}

	.error-bubble {
		border-color: #7a1e1e;
		background: #2a1a1a;
	}

	.message-content {
		display: inline;
	}

	.message-content.dimmed {
		opacity: 0.6;
	}

	/* Blinking cursor for active streaming */
	.cursor {
		display: inline-block;
		margin-left: 1px;
		color: #4a9eff;
		animation: blink 1s step-end infinite;
	}

	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0;
		}
	}

	/* Amber cancelled/interrupted badge (D-06) */
	.cancelled-badge {
		display: inline-block;
		margin-left: 6px;
		font-size: 11px;
		font-weight: 600;
		color: #e0a020;
		vertical-align: middle;
	}

	.retry-button {
		display: inline-block;
		margin-left: 8px;
		border: none;
		background: transparent;
		color: #4a9eff;
		padding: 0;
		font-size: 12px;
		font-weight: 500;
		cursor: pointer;
		vertical-align: middle;
	}

	.retry-button:hover {
		text-decoration: underline;
	}

	.retry-button:focus-visible {
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}
</style>
