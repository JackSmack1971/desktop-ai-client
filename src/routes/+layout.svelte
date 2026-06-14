<script lang="ts">
	import { onMount } from 'svelte';
	import { surfaceStore } from '$lib/stores/surface';

	interface Props {
		children?: import('svelte').Snippet;
	}
	let { children }: Props = $props();

	// Hydrate the active surface from backend state on first render.
	// The store handles the IPC call; it falls back to 'chat' if the call fails
	// so the shell always renders even if the backend is not yet connected.
	onMount(() => {
		surfaceStore.hydrate().catch((e) => console.error('surface hydration failed', e));
	});
</script>

{@render children?.()}
