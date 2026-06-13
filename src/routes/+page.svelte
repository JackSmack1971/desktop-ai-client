<script lang="ts">
	import { surfaceStore } from '$lib/stores/surface';
	import ChatSurface from '$lib/components/surfaces/ChatSurface.svelte';
	import HistorySurface from '$lib/components/surfaces/HistorySurface.svelte';
	import SettingsSurface from '$lib/components/surfaces/SettingsSurface.svelte';
	import ArtifactsSurface from '$lib/components/surfaces/ArtifactsSurface.svelte';

	// Svelte 5 rune: derive the active surface from the store's reactive state.
	let activeSurface = $derived(surfaceStore.surface);
</script>

<div class="surface-container">
	{#if activeSurface === 'chat'}
		<ChatSurface />
	{:else if activeSurface === 'history'}
		<HistorySurface />
	{:else if activeSurface === 'settings'}
		<SettingsSurface />
	{:else if activeSurface === 'artifacts'}
		<ArtifactsSurface />
	{:else}
		<ChatSurface />
	{/if}
</div>

<style>
	.surface-container {
		display: contents;
	}
</style>
