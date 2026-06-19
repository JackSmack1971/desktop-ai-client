<script lang="ts">
	/**
	 * WorkspaceShell – the complete desktop workspace layout with nav, content,
	 * and status bar.
	 *
	 * Composes SurfaceRail, SurfacePanel, and StatusRegion into a single layout
	 * unit. The shell owns focus management: the "skip to content" link lets
	 * keyboard users bypass the nav rail on every surface switch.
	 *
	 * ARIA structure:
	 *   role="application" (desktop application, not a document)
	 *     nav[aria-label="Surface navigation"]  → SurfaceRail (tablist pattern)
	 *     main[id="main-content"]               → SurfacePanel (tabpanel)
	 *     div[role="status"]                    → StatusRegion (live region)
	 *
	 * Keyboard contract:
	 *   - Tab navigates between the skip link, rail buttons, and panel.
	 *   - Arrow keys move focus within the rail (roving tabindex pattern).
	 *   - Enter / Space activate a rail button and move focus to the panel.
	 *   - The panel itself is focusable so Enter on a rail button can jump into it.
	 */

	import SurfaceRail from './SurfaceRail.svelte';
	import StatusRegion from './StatusRegion.svelte';
	import { surfaceStore, type Surface } from '$lib/stores/surface';

	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();

	const surfaceLabels: Record<Surface, string> = {
		chat: 'Chat',
		history: 'History',
		settings: 'Settings',
		artifacts: 'Artifacts',
	};

	let activeSurface = $derived(surfaceStore.surface);
	let surfaceLabel = $derived(surfaceLabels[activeSurface] ?? 'Chat');
	let statusMessage = $derived(`${surfaceLabel} surface active`);
</script>

<!-- Skip-to-content link for keyboard users who want to bypass the nav rail. -->
<a class="skip-link" href="#main-content">Skip to content</a>

<div class="workspace-shell" role="application" aria-label="Desktop AI Client">
	<!-- Side navigation rail: drives surface switching. Tab-list pattern. -->
	<nav class="workspace-shell__rail" aria-label="Surface navigation">
		<SurfaceRail />
	</nav>

	<!-- Main content area: displays the active surface inside a named tabpanel. -->
	<div class="workspace-shell__body">
		<main
			class="workspace-shell__content"
			id="main-content"
			tabindex="-1"
			aria-label={surfaceLabel}
		>
			{@render children?.()}
		</main>

		<!-- Status bar: visible and AT-announced shell state. -->
		<StatusRegion
			message={statusMessage}
			loading={surfaceStore.loading}
			error={surfaceStore.error}
		/>
	</div>
</div>

<style>
	.skip-link {
		position: absolute;
		top: -100%;
		left: 0;
		z-index: 9999;
		padding: 8px 16px;
		background: #4a9eff;
		color: #fff;
		font-size: 14px;
		font-weight: 600;
		text-decoration: none;
		border-radius: 0 0 4px 0;
		transition: top 0.1s;
	}

	.skip-link:focus {
		top: 0;
	}

	.workspace-shell {
		display: flex;
		flex-direction: row;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
		background-color: #0f0f0f;
		color: #e0e0e0;
		font-family:
			system-ui,
			-apple-system,
			sans-serif;
	}

	.workspace-shell__rail {
		flex: 0 0 56px;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 8px 0;
		background-color: #1a1a1a;
		border-right: 1px solid #2a2a2a;
		gap: 4px;
	}

	.workspace-shell__body {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		min-width: 0;
	}

	.workspace-shell__content {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		outline: none;
	}
</style>
