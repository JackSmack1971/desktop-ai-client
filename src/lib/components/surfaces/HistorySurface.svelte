<script lang="ts">
	/**
	 * HistorySurface — conversation history browser.
	 *
	 * Replaces the Phase 1 scaffold with a fully wired surface that composes
	 * SearchBar and ConversationList, driven by historyStore. Loads history
	 * on mount via historyStore.load(). Conversation clicks navigate to the
	 * Chat surface via surfaceStore.setSurface('chat') (D-10).
	 */

	import { onMount } from 'svelte';
	import { historyStore } from '$lib/stores/history';
	import { surfaceStore } from '$lib/stores/surface';
	import SearchBar from '$lib/components/history/SearchBar.svelte';
	import ConversationList from '$lib/components/history/ConversationList.svelte';

	onMount(() => {
		void historyStore.load();
	});

	function handleQuery(q: string): void {
		void historyStore.search(q);
	}

	function handleSelect(id: string): void {
		historyStore.loadConversation(id);
		void surfaceStore.setSurface('chat');
	}

	function handleDelete(id: string): void {
		void historyStore.deleteConversation(id);
	}
</script>

<div class="surface history-surface" role="region" aria-label="History">
	<header class="surface-header">
		<h1 class="surface-title">History</h1>
	</header>
	<div class="surface-body history-body">
		<div class="search-area">
			<SearchBar onquery={handleQuery} />
		</div>
		{#if historyStore.error}
			<p class="history-error" role="alert">
				Could not load history. Check the app connection and try again.
			</p>
		{/if}
		<ConversationList
			conversations={historyStore.conversations}
			loading={historyStore.loading}
			ondelete={handleDelete}
			onselect={handleSelect}
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
		overflow: hidden;
	}

	.history-body {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}

	.search-area {
		padding: 16px 24px 8px;
		background: #1a1a1a;
		border-bottom: 1px solid #2a2a2a;
		flex: 0 0 auto;
	}

	.history-error {
		padding: 8px 16px;
		color: #cc4444;
		font-size: 14px;
		background: #2a1a1a;
		border-bottom: 1px solid #7a1e1e;
		margin: 0;
	}
</style>
