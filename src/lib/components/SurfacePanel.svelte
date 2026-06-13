<script lang="ts">
	/**
	 * SurfacePanel – ARIA panel wrapper for the currently active surface.
	 *
	 * Provides the tabpanel role to match the tab-like navigation of the
	 * SurfaceRail, and uses aria-label to announce which surface is displayed
	 * to assistive technology whenever the active surface changes.
	 *
	 * The panel is the direct counterpart to the SurfaceRail's button set.
	 * Together they form a coherent tab-like navigation pattern:
	 *   - Rail buttons: role=tab, aria-selected, aria-controls=surface-panel
	 *   - Panel: role=tabpanel, aria-labelledby=<active-tab-id>
	 *
	 * Keyboard: focus lands in the panel after surface switch via
	 * AppShell's skip-to-content mechanism.
	 */

	interface Props {
		/** Human-readable name of the currently displayed surface. */
		surfaceLabel: string;
		children?: import('svelte').Snippet;
	}

	let { surfaceLabel, children }: Props = $props();
</script>

<div
	id="surface-panel"
	class="surface-panel"
	role="tabpanel"
	aria-label={surfaceLabel}
	tabindex="-1"
>
	{@render children?.()}
</div>

<style>
	.surface-panel {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		overflow: hidden;
		outline: none;
	}
</style>
