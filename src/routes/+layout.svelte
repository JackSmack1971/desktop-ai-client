<script lang="ts">
	import { onMount } from 'svelte';
	import { surfaceStore } from '$lib/stores/surface';
	import AppShell from '$lib/components/AppShell.svelte';

	// Hydrate the active surface from backend state on first render.
	// The store handles the IPC call; it falls back to 'chat' if the call fails
	// so the shell always renders even if the backend is not yet connected.
	onMount(() => {
		surfaceStore.hydrate().catch((e) => console.error('surface hydration failed', e));
	});
</script>

<AppShell>
	<slot />
</AppShell>
