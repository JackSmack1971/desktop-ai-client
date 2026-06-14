<script lang="ts">
	/**
	 * ChatInput — message composition area with send and cancel controls.
	 *
	 * Renders a textarea and submit button. On Enter (without Shift) submits if
	 * non-empty. Cancel button appears when `showCancel` is true (D-05: cancel
	 * button is anchored in the input area, not inside the message bubble).
	 */

	interface Props {
		onsubmit: (text: string) => void;
		disabled: boolean;
		showCancel: boolean;
		oncancel: () => void;
	}

	let { onsubmit, disabled, showCancel, oncancel }: Props = $props();

	let text = $state('');

	function handleSubmit(e: Event): void {
		e.preventDefault();
		const trimmed = text.trim();
		if (!trimmed || disabled) return;
		onsubmit(trimmed);
		text = '';
	}

	function handleKeydown(e: KeyboardEvent): void {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSubmit(e);
		}
		// Shift+Enter: allow default (inserts newline in textarea).
	}
</script>

<form class="chat-input-form" onsubmit={handleSubmit}>
	<textarea
		class="chat-textarea"
		bind:value={text}
		onkeydown={handleKeydown}
		{disabled}
		placeholder="Type a message…"
		rows={1}
		aria-label="Message input"
	></textarea>

	<div class="chat-input-actions">
		{#if showCancel}
			<button
				type="button"
				class="btn-cancel"
				onclick={oncancel}
				aria-label="Cancel streaming response"
			>
				Cancel
			</button>
		{/if}

		<button
			type="submit"
			class="btn-send"
			{disabled}
			aria-label="Send message"
		>
			{#if disabled}
				Sending…
			{:else}
				Send
			{/if}
		</button>
	</div>
</form>

<style>
	.chat-input-form {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px 16px;
		border-top: 1px solid #2a2a2a;
		background: #1a1a1a;
	}

	.chat-textarea {
		width: 100%;
		min-height: 40px;
		max-height: 160px;
		padding: 10px 12px;
		background: #242424;
		border: 1px solid #2a2a2a;
		border-radius: 6px;
		color: #e0e0e0;
		font-size: 14px;
		line-height: 1.5;
		resize: vertical;
		outline: none;
		box-sizing: border-box;
	}

	.chat-textarea:focus {
		border-color: #4a9eff;
		box-shadow: 0 0 0 2px rgba(74, 158, 255, 0.25);
	}

	.chat-textarea:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.chat-input-actions {
		display: flex;
		gap: 8px;
		justify-content: flex-end;
	}

	.btn-send,
	.btn-cancel {
		padding: 8px 16px;
		border-radius: 6px;
		font-size: 14px;
		font-weight: 500;
		cursor: pointer;
		border: none;
		transition: background 0.15s;
	}

	.btn-send {
		background: #4a9eff;
		color: #fff;
	}

	.btn-send:hover:not(:disabled) {
		background: #3a8eef;
	}

	.btn-send:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.btn-cancel {
		background: #3a3a2a;
		color: #e0a020;
		border: 1px solid #5a5a2a;
	}

	.btn-cancel:hover {
		background: #4a4a2a;
	}
</style>
