<script lang="ts">
	/**
	 * ConversationList — scrollable list of ConversationRow items.
	 *
	 * Renders loading, empty, and populated states.
	 * id="conversation-list" links to SearchBar's aria-controls attribute.
	 */

	import type { ConversationSummary } from '$lib/stores/history';
	import ConversationRow from '$lib/components/history/ConversationRow.svelte';

	interface Props {
		conversations: ConversationSummary[];
		loading: boolean;
		ondelete: (id: string) => void;
		onselect: (id: string) => void;
	}

	let { conversations, loading, ondelete, onselect }: Props = $props();
</script>

<div
	class="conversation-list"
	role="list"
	id="conversation-list"
	aria-live="polite"
	aria-busy={loading}
>
	{#if loading}
		<p class="list-status" aria-live="polite">Loading conversations…</p>
	{:else if conversations.length === 0}
		<div class="empty-state">
			<p class="empty-heading">No conversations yet</p>
			<p class="empty-body">
				Your conversations will appear here after your first chat. Start a new chat to begin.
			</p>
		</div>
	{:else}
		{#each conversations as conv (conv.id)}
			<ConversationRow
				{conv}
				ondelete={() => ondelete(conv.id)}
				onselect={() => onselect(conv.id)}
			/>
		{/each}
	{/if}
</div>

<style>
	.conversation-list {
		display: flex;
		flex-direction: column;
		gap: 0;
		overflow-y: auto;
		flex: 1 1 auto;
	}

	.list-status {
		text-align: center;
		color: #888;
		font-size: 14px;
		padding: 24px 16px;
		margin: 0;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 48px 24px;
		text-align: center;
		gap: 8px;
	}

	.empty-heading {
		margin: 0;
		color: #888;
		font-size: 14px;
		font-weight: 400;
	}

	.empty-body {
		margin: 0;
		color: #666;
		font-size: 14px;
		line-height: 1.5;
	}
</style>
