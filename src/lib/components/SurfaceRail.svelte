<script lang="ts">
	/**
	 * SurfaceRail – icon navigation rail for switching between major surfaces.
	 *
	 * Each button calls `surfaceStore.setSurface()` which persists the selection
	 * to the backend through typed IPC. No browser storage is used.
	 *
	 * Accessibility – tab-list pattern with roving tabindex:
	 *   - The rail has role="tablist" so the relationship to the surface panel
	 *     (tabpanel) is expressed in the ARIA tree.
	 *   - Only the active button is in the natural tab order (tabindex=0).
	 *     All others are removed (tabindex=-1). This is the "roving tabindex"
	 *     pattern recommended by ARIA APG for tab sets.
	 *   - Arrow Up / Arrow Down moves focus within the rail.
	 *   - Enter and Space activate the focused item.
	 *   - aria-selected (not aria-current) is the correct attribute for tabs.
	 *   - The rail is labelled so screen readers describe it correctly.
	 */
	import { surfaceStore, type Surface } from '$lib/stores/surface';

	const surfaces: Array<{ id: Surface; label: string; icon: string }> = [
		{ id: 'chat',      label: 'Chat',      icon: '💬' },
		{ id: 'history',   label: 'History',   icon: '📋' },
		{ id: 'settings',  label: 'Settings',  icon: '⚙️' },
		{ id: 'artifacts', label: 'Artifacts', icon: '📦' },
	];

	let activeSurface = $derived(surfaceStore.surface);

	/** The index in `surfaces` of the button that currently has DOM focus. */
	let focusedIndex = $state(
		surfaces.findIndex((s) => s.id === activeSurface) || 0
	);

	/** Button element refs — populated by bind:this in {#each}. */
	let buttonRefs: Array<HTMLButtonElement | null> = $state(
		new Array(surfaces.length).fill(null)
	);

	function activate(surface: Surface) {
		surfaceStore.setSurface(surface);
		// Move focus to the main content area so keyboard users land in the panel.
		const main = document.getElementById('main-content');
		main?.focus();
	}

	function moveFocus(delta: -1 | 1) {
		const next = (focusedIndex + delta + surfaces.length) % surfaces.length;
		focusedIndex = next;
		buttonRefs[next]?.focus();
	}

	function handleKeydown(event: KeyboardEvent, index: number) {
		switch (event.key) {
			case 'ArrowDown':
			case 'ArrowRight':
				event.preventDefault();
				moveFocus(1);
				break;
			case 'ArrowUp':
			case 'ArrowLeft':
				event.preventDefault();
				moveFocus(-1);
				break;
			case 'Enter':
			case ' ':
				event.preventDefault();
				activate(surfaces[index].id);
				break;
			case 'Home':
				event.preventDefault();
				focusedIndex = 0;
				buttonRefs[0]?.focus();
				break;
			case 'End':
				event.preventDefault();
				focusedIndex = surfaces.length - 1;
				buttonRefs[surfaces.length - 1]?.focus();
				break;
		}
	}

	function handleClick(surface: Surface, index: number) {
		focusedIndex = index;
		activate(surface);
	}
</script>

<div
	role="tablist"
	aria-label="Workspace surfaces"
	aria-orientation="vertical"
>
	{#each surfaces as { id, label, icon }, index}
		<button
			bind:this={buttonRefs[index]}
			class="rail-button"
			class:rail-button--active={activeSurface === id}
			role="tab"
			id="tab-{id}"
			aria-label={label}
			aria-selected={activeSurface === id}
			aria-controls="surface-panel"
			tabindex={focusedIndex === index ? 0 : -1}
			onclick={() => handleClick(id, index)}
			onkeydown={(e) => handleKeydown(e, index)}
			type="button"
		>
			<span class="rail-button__icon" aria-hidden="true">{icon}</span>
			<span class="rail-button__label">{label}</span>
		</button>
	{/each}

	{#if surfaceStore.loading}
		<div class="rail-status" aria-live="polite" aria-label="Loading surface preferences">
			<span aria-hidden="true">…</span>
		</div>
	{/if}

	{#if surfaceStore.error}
		<div class="rail-status rail-status--error" role="alert" aria-live="assertive">
			<span class="sr-only">Surface preference error: offline mode</span>
		</div>
	{/if}
</div>

<style>
	.rail-button {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border: none;
		border-radius: 8px;
		background: transparent;
		color: #888;
		cursor: pointer;
		padding: 0;
		gap: 2px;
		transition: background-color 0.15s ease, color 0.15s ease;
		position: relative;
	}

	.rail-button:hover {
		background-color: #2a2a2a;
		color: #e0e0e0;
	}

	.rail-button:focus-visible {
		/* Visible focus indicator required per accessibility release gate */
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}

	.rail-button--active {
		background-color: #2a2a2a;
		color: #4a9eff;
	}

	.rail-button--active::before {
		/* Active indicator bar on the left edge */
		content: '';
		position: absolute;
		left: -8px;
		top: 50%;
		transform: translateY(-50%);
		width: 3px;
		height: 24px;
		background-color: #4a9eff;
		border-radius: 0 2px 2px 0;
	}

	.rail-button__icon {
		font-size: 18px;
		line-height: 1;
	}

	.rail-button__label {
		font-size: 9px;
		line-height: 1;
		letter-spacing: 0.02em;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 44px;
	}

	.rail-status {
		margin-top: auto;
		padding: 4px;
		font-size: 10px;
		color: #666;
	}

	.rail-status--error {
		color: #cc4444;
	}

	/* Screen-reader only text helper */
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}
</style>
