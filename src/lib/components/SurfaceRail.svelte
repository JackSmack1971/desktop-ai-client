<script lang="ts">
	/**
	 * SurfaceRail – icon navigation rail for switching between major surfaces.
	 *
	 * Each button calls `surfaceStore.setSurface()` which persists the selection
	 * to the backend through typed IPC. No browser storage is used.
	 *
	 * Accessibility: buttons have aria-label and aria-current so keyboard users
	 * and screen readers can identify the active surface.
	 */
	import { surfaceStore, type Surface } from '$lib/stores/surface';

	const surfaces: Array<{ id: Surface; label: string; icon: string }> = [
		{ id: 'chat',      label: 'Chat',      icon: '💬' },
		{ id: 'history',   label: 'History',   icon: '📋' },
		{ id: 'settings',  label: 'Settings',  icon: '⚙️' },
		{ id: 'artifacts', label: 'Artifacts', icon: '📦' }
	];

	// Svelte 5 rune: derive which surface is currently active.
	let activeSurface = $derived(surfaceStore.surface);

	function handleClick(surface: Surface) {
		surfaceStore.setSurface(surface);
	}

	function handleKeydown(event: KeyboardEvent, surface: Surface) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			surfaceStore.setSurface(surface);
		}
	}
</script>

{#each surfaces as { id, label, icon }}
	<button
		class="rail-button"
		class:rail-button--active={activeSurface === id}
		aria-label={label}
		aria-current={activeSurface === id ? 'page' : undefined}
		title={label}
		onclick={() => handleClick(id)}
		onkeydown={(e) => handleKeydown(e, id)}
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
