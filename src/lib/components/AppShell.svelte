<script lang="ts">
	/**
	 * AppShell – root layout component for the workspace desktop shell.
	 *
	 * Renders the side rail navigation and the main content slot.
	 * The rail drives surface changes through the backend-owned surface store;
	 * it never writes to browser storage.
	 *
	 * Accessibility: focus management and ARIA landmarks are required per the
	 * adversarial architecture spec's accessibility release gate.
	 */
	import SurfaceRail from './SurfaceRail.svelte';

	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="app-shell" role="application" aria-label="Desktop AI Client">
	<!-- Side navigation rail: drives surface switching through typed IPC -->
	<nav class="app-shell__rail" aria-label="Surface navigation">
		<SurfaceRail />
	</nav>

	<!-- Main content area: renders the active surface component -->
	<main class="app-shell__content" id="main-content" tabindex="-1">
		{@render children?.()}
	</main>
</div>

<style>
	.app-shell {
		display: flex;
		flex-direction: row;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
		background-color: #0f0f0f;
		color: #e0e0e0;
		font-family: system-ui, -apple-system, sans-serif;
	}

	.app-shell__rail {
		flex: 0 0 56px;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 8px 0;
		background-color: #1a1a1a;
		border-right: 1px solid #2a2a2a;
		gap: 4px;
	}

	.app-shell__content {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		outline: none;
	}
</style>
