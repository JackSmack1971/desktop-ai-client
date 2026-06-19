<script lang="ts">
	/**
	 * ConversationRow — single conversation entry with inline delete confirmation.
	 *
	 * Security: snippet content is rendered as plain text — NOT with @html.
	 * FTS5 snippet() markers like <b>term</b> appear as literal characters,
	 * preventing XSS via stored message content (T-03-16).
	 */

	import type { ConversationSummary } from '$lib/stores/history';

	interface Props {
		conv: ConversationSummary;
		ondelete: () => void;
		onselect: () => void;
	}

	let { conv, ondelete, onselect }: Props = $props();

	let showConfirm = $state(false);
	let deleteError = $state<string | null>(null);
	let deleting = $state(false);
	let deleteButtonEl = $state<HTMLButtonElement | undefined>(undefined);

	function handleRowClick(): void {
		if (!showConfirm) onselect();
	}

	function handleDeleteClick(e: MouseEvent): void {
		e.stopPropagation();
		showConfirm = true;
		deleteError = null;
	}

	function handleConfirmDelete(): void {
		deleting = true;
		deleteError = null;
		ondelete();
	}

	function handleCancelDelete(): void {
		showConfirm = false;
		deleteError = null;
		deleteButtonEl?.focus();
	}

	function handleKeydown(e: KeyboardEvent): void {
		if (e.key === 'Escape' && showConfirm) {
			handleCancelDelete();
		}
	}

	/**
	 * Format an ISO datetime string as a relative time label.
	 * Returns human-readable strings like "just now", "5 minutes ago", etc.
	 */
	function relativeTime(isoString: string): string {
		const diffMs = Date.now() - new Date(isoString).getTime();
		const diffSec = Math.floor(diffMs / 1000);
		if (diffSec < 60) return 'just now';
		const diffMin = Math.floor(diffSec / 60);
		if (diffMin < 60) return `${diffMin} minute${diffMin === 1 ? '' : 's'} ago`;
		const diffHours = Math.floor(diffMin / 60);
		if (diffHours < 24)
			return `${diffHours} hour${diffHours === 1 ? '' : 's'} ago`;
		const diffDays = Math.floor(diffHours / 24);
		return `${diffDays} day${diffDays === 1 ? '' : 's'} ago`;
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="conversation-row"
	class:deleting
	onkeydown={handleKeydown}
	role="listitem"
>
	<button
		class="row-body"
		onclick={handleRowClick}
		aria-label="{conv.title} — {relativeTime(conv.updatedAt)}"
	>
		<span class="conv-title">{conv.title}</span>
		{#if conv.snippet}
			<span class="conv-snippet">{conv.snippet}</span>
		{:else}
			<span class="conv-model">{conv.model}</span>
		{/if}
		{#if conv.status === 'incomplete'}
			<span class="status-badge status-incomplete">Incomplete</span>
		{/if}
		<span class="conv-timestamp">{relativeTime(conv.updatedAt)}</span>
	</button>
	<button
		bind:this={deleteButtonEl}
		class="delete-trigger"
		onclick={handleDeleteClick}
		aria-label="Delete conversation: {conv.title}"
	>
		Delete conversation
	</button>
	{#if showConfirm}
		<div
			class="delete-confirm"
			role="alertdialog"
			aria-label="Confirm deletion"
			aria-modal="true"
		>
			<p class="confirm-heading">Delete this conversation?</p>
			<p class="confirm-body">This cannot be undone.</p>
			<div class="confirm-actions">
				<button
					class="confirm-btn"
					onclick={handleConfirmDelete}
					disabled={deleting}
				>
					Confirm delete
				</button>
				<button class="cancel-link" onclick={handleCancelDelete}>Cancel</button>
			</div>
			{#if deleteError}
				<p class="delete-error" role="alert">{deleteError}</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	.conversation-row {
		position: relative;
		background: #242424;
		border-bottom: 1px solid #2a2a2a;
		min-height: 56px;
	}

	.conversation-row:hover {
		background: #1a1a1a;
		transition: background 0.15s ease;
	}

	.conversation-row.deleting {
		opacity: 0.6;
		pointer-events: none;
	}

	.row-body {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		min-height: 56px;
		padding: 8px 16px;
		background: transparent;
		border: none;
		cursor: pointer;
		text-align: left;
		font-family:
			system-ui,
			-apple-system,
			sans-serif;
		color: inherit;
		box-sizing: border-box;
	}

	.row-body:focus-visible {
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}

	.conv-title {
		flex: 1 1 auto;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: #e0e0e0;
		font-size: 14px;
		font-weight: 400;
	}

	.conv-model {
		flex: 0 0 auto;
		color: #888;
		font-size: 12px;
		white-space: nowrap;
	}

	.conv-snippet {
		flex: 0 1 auto;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: #888;
		font-size: 12px;
	}

	.conv-timestamp {
		flex: 0 0 auto;
		color: #888;
		font-size: 12px;
		white-space: nowrap;
	}

	.status-badge {
		flex: 0 0 auto;
		font-size: 11px;
		font-weight: 600;
		white-space: nowrap;
	}

	.status-incomplete {
		color: #e0a020;
	}

	.delete-trigger {
		position: absolute;
		top: 50%;
		right: 16px;
		transform: translateY(-50%);
		background: transparent;
		border: none;
		color: #888;
		font-size: 12px;
		cursor: pointer;
		padding: 4px 8px;
		font-family:
			system-ui,
			-apple-system,
			sans-serif;
		opacity: 0;
		transition: opacity 0.15s ease;
	}

	.conversation-row:hover .delete-trigger,
	.delete-trigger:focus-visible {
		opacity: 1;
	}

	.delete-trigger:focus-visible {
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}

	.delete-confirm {
		background: #2a1a1a;
		border-top: 1px solid #7a1e1e;
		padding: 12px 16px;
	}

	.confirm-heading {
		margin: 0 0 4px;
		color: #e0e0e0;
		font-size: 14px;
		font-weight: 400;
	}

	.confirm-body {
		margin: 0 0 12px;
		color: #888;
		font-size: 14px;
	}

	.confirm-actions {
		display: flex;
		gap: 12px;
		align-items: center;
	}

	.confirm-btn {
		background: #2a1a1a;
		border: 1px solid #7a1e1e;
		color: #cc4444;
		font-size: 14px;
		padding: 6px 12px;
		border-radius: 4px;
		cursor: pointer;
		font-family:
			system-ui,
			-apple-system,
			sans-serif;
	}

	.confirm-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.confirm-btn:focus-visible {
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}

	.cancel-link {
		background: transparent;
		border: none;
		color: #888;
		font-size: 14px;
		cursor: pointer;
		padding: 6px 0;
		font-family:
			system-ui,
			-apple-system,
			sans-serif;
	}

	.cancel-link:focus-visible {
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}

	.delete-error {
		margin: 8px 0 0;
		color: #cc4444;
		font-size: 14px;
	}
</style>
