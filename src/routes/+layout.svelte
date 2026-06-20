<script lang="ts">
	import { onMount } from 'svelte';
	import { surfaceStore } from '$lib/stores/surface';

	interface Props {
		children?: import('svelte').Snippet;
	}
	let { children }: Props = $props();

	// Hydrate the active surface from backend state on first render.
	// The store handles the IPC call and surfaces backend errors itself; this
	// layout simply kicks off hydration and renders whatever readiness state the
	// backend reports.
	onMount(() => {
		void surfaceStore.hydrate();
	});
</script>

{@render children?.()}
