<script lang="ts">
	import { surfaceStore, type Surface } from '$lib/stores/surface';
	import WorkspaceShell from '$lib/components/WorkspaceShell.svelte';
	import SurfacePanel from '$lib/components/SurfacePanel.svelte';
	import ChatSurface from '$lib/components/surfaces/ChatSurface.svelte';
	import HistorySurface from '$lib/components/surfaces/HistorySurface.svelte';
	import SettingsSurface from '$lib/components/surfaces/SettingsSurface.svelte';
	import ArtifactsSurface from '$lib/components/surfaces/ArtifactsSurface.svelte';

	const surfaceLabels: Record<Surface, string> = {
		chat: 'Chat',
		history: 'History',
		settings: 'Settings',
		artifacts: 'Artifacts',
	};

	// Svelte 5 rune: derive the active surface from the store's reactive state.
	let activeSurface = $derived(surfaceStore.surface);
	let surfaceLabel = $derived(surfaceLabels[activeSurface] ?? 'Chat');
</script>

<WorkspaceShell>
	<SurfacePanel {surfaceLabel}>
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
	</SurfacePanel>
</WorkspaceShell>
