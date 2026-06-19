<script lang="ts">
	/**
	 * StreamingBubble — the transitional assistant message bubble.
	 *
	 * Handles the hybrid loading UX (D-05):
	 * - `loading === true`: renders a skeleton/thinking indicator (three
	 *   animated dots) so the user knows the request is in-flight.
	 * - `loading === false` and `content` accumulating: renders partial text
	 *   with a blinking cursor.
	 *
	 * The transition from skeleton to content happens on first non-empty content.
	 * `aria-live="polite"` ensures screen readers announce updates without
	 * interrupting the user.
	 */

	interface Props {
		content: string;
		loading: boolean;
	}

	let { content, loading }: Props = $props();

	let hasContent = $derived(content.length > 0);
</script>

<div class="streaming-bubble" aria-label="Assistant is responding">
	{#if loading && !hasContent}
		<!-- Thinking indicator: three animated dots -->
		<div
			class="thinking"
			role="status"
			aria-label="Thinking…"
			aria-live="polite"
		>
			<span class="dot" aria-hidden="true"></span>
			<span class="dot" aria-hidden="true"></span>
			<span class="dot" aria-hidden="true"></span>
			<span class="visually-hidden">Thinking…</span>
		</div>
	{:else}
		<!-- Streaming content with cursor -->
		<div class="streaming-content" aria-live="polite">
			<span class="content-text">{content}</span>
			<span class="cursor" aria-hidden="true">|</span>
		</div>
	{/if}
</div>

<style>
	.streaming-bubble {
		max-width: 75%;
		padding: 10px 14px;
		background: #242424;
		border: 1px solid #2a2a2a;
		border-radius: 12px;
		border-bottom-left-radius: 3px;
		font-size: 14px;
		line-height: 1.6;
		color: #e0e0e0;
		white-space: pre-wrap;
		word-break: break-word;
		margin: 4px 16px;
	}

	/* Thinking dots animation */
	.thinking {
		display: flex;
		align-items: center;
		gap: 4px;
		height: 24px;
	}

	.dot {
		width: 6px;
		height: 6px;
		background: #666;
		border-radius: 50%;
		animation: pulse 1.4s ease-in-out infinite;
	}

	.dot:nth-child(2) {
		animation-delay: 0.2s;
	}
	.dot:nth-child(3) {
		animation-delay: 0.4s;
	}

	@keyframes pulse {
		0%,
		80%,
		100% {
			transform: scale(0.8);
			opacity: 0.5;
		}
		40% {
			transform: scale(1);
			opacity: 1;
		}
	}

	.streaming-content {
		display: inline;
	}

	.content-text {
		display: inline;
	}

	/* Blinking cursor while streaming */
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

	/* Accessible screen reader text */
	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
</style>
