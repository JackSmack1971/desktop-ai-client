<script lang="ts">
	import { tick } from 'svelte';
	import { artifactsStore } from '$lib/stores/artifacts';

	let reloadButton = $state<HTMLButtonElement | null>(null);
	let stopButton = $state<HTMLButtonElement | null>(null);

	async function handleReload(): Promise<void> {
		await artifactsStore.reload();
	}

	async function handleStop(): Promise<void> {
		await artifactsStore.dismiss();
		await tick();
		if (reloadButton) {
			reloadButton?.focus();
		}
	}

	let showReload = $derived(
		artifactsStore.hasArtifact && artifactsStore.state !== 'idle',
	);
	let showStop = $derived(
		artifactsStore.state === 'ready' || artifactsStore.state === 'loading',
	);
	let contentTypeLabel = $derived(artifactsStore.contentTypeLabel);
</script>

<div class="surface artifacts-surface" role="region" aria-label="Artifacts">
	<header class="surface-header">
		<div class="surface-header__title-group">
			<h1 class="surface-title">Artifacts</h1>
			{#if contentTypeLabel}
				<span
					class="artifact-badge"
					class:artifact-badge--html={artifactsStore.contentType?.type ===
						'Html'}
					aria-label={`${contentTypeLabel} artifact`}
				>
					{contentTypeLabel}
				</span>
			{/if}
		</div>

		<div class="surface-toolbar" role="toolbar" aria-label="Artifact controls">
			{#if showReload}
				<button
					bind:this={reloadButton}
					class="toolbar-button toolbar-button--reload"
					disabled={artifactsStore.isLoading}
					onclick={() => void handleReload()}
					type="button"
					aria-label="Reload artifact"
				>
					Reload
				</button>
			{/if}

			{#if showStop}
				<button
					bind:this={stopButton}
					class="toolbar-button toolbar-button--stop"
					disabled={artifactsStore.isLoading}
					onclick={() => void handleStop()}
					type="button"
					aria-label="Stop artifact preview"
				>
					Stop
				</button>
			{/if}
		</div>
	</header>

	<div class="surface-body">
		{#if artifactsStore.state === 'idle'}
			<div class="state-panel state-panel--empty">
				<p class="state-heading">No artifact yet</p>
				<p class="state-copy">
					Send a message asking Claude to generate HTML, SVG, or code. The
					result will appear here.
				</p>
			</div>
		{:else if artifactsStore.state === 'loading'}
			<div
				class="state-panel state-panel--loading"
				role="status"
				aria-live="polite"
			>
				<p class="state-heading">Loading artifact…</p>
			</div>
		{:else if artifactsStore.state === 'error'}
			<div
				class="state-panel state-panel--error"
				role="alert"
				aria-live="assertive"
			>
				<p class="state-heading">
					Could not load artifact. Check the app connection and try reloading.
				</p>
				<p class="state-copy">
					Artifact could not be displayed safely. Reload to try again.
				</p>
			</div>
		{:else if artifactsStore.state === 'dismissed'}
			<div
				class="state-panel state-panel--dismissed"
				role="status"
				aria-live="polite"
			>
				<p class="state-heading">Artifact dismissed</p>
				<p class="state-copy">
					Reload to restore the preview from the backend.
				</p>
			</div>
		{:else if artifactsStore.isReady}
			<iframe
				class="artifact-frame"
				title="Artifact preview"
				sandbox=""
				srcdoc={artifactsStore.preview}
				role="document"
			></iframe>
		{:else}
			<div
				class="state-panel state-panel--error"
				role="alert"
				aria-live="assertive"
			>
				<p class="state-heading">Artifact could not be displayed safely.</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.surface {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: 100%;
	}

	.surface-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 16px;
		padding: 16px 24px;
		border-bottom: 1px solid #2a2a2a;
		flex: 0 0 auto;
		background: #1a1a1a;
	}

	.surface-header__title-group {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}

	.surface-title {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
		color: #e0e0e0;
	}

	.artifact-badge {
		display: inline-flex;
		align-items: center;
		padding: 2px 8px;
		border-radius: 4px;
		background: #2a2a2a;
		color: #888;
		font-size: 13px;
		line-height: 1.4;
		white-space: nowrap;
	}

	.artifact-badge--html {
		color: #4a9eff;
	}

	.surface-toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 0 0 auto;
	}

	.toolbar-button {
		min-width: 88px;
		height: 48px;
		padding: 0 16px;
		border: 1px solid #2a2a2a;
		border-radius: 8px;
		background: #2a2a2a;
		color: #e0e0e0;
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			color 0.15s ease,
			border-color 0.15s ease;
	}

	.toolbar-button:hover:not(:disabled) {
		background: #333;
	}

	.toolbar-button:focus-visible {
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}

	.toolbar-button:disabled {
		cursor: not-allowed;
		opacity: 0.55;
	}

	.toolbar-button--reload:hover:not(:disabled) {
		border-color: #4a9eff;
	}

	.toolbar-button--stop {
		color: #cc4444;
	}

	.surface-body {
		flex: 1 1 auto;
		display: flex;
		overflow: hidden;
		background: #0f0f0f;
	}

	.artifact-frame {
		flex: 1 1 auto;
		width: 100%;
		height: 100%;
		border: 0;
		background: #0f0f0f;
	}

	.state-panel {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 24px;
		text-align: center;
		gap: 8px;
	}

	.state-heading {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
		color: #e0e0e0;
	}

	.state-copy {
		margin: 0;
		font-size: 14px;
		line-height: 1.5;
		color: #666;
		max-width: 48rem;
	}

	.state-panel--error .state-heading {
		color: #cc4444;
	}

	.state-panel--loading .state-heading {
		color: #4a9eff;
	}

	.state-panel--dismissed .state-heading {
		color: #888;
	}
</style>
