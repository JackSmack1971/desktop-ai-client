<script lang="ts">
	/**
	 * SettingsSurface — credential management only.
	 *
	 * Phase 4 uses this component to manage the OpenRouter API key without
	 * ever rendering the key value back into the DOM.
	 */

	import { onMount, tick } from 'svelte';
	import { privacyStore } from '$lib/stores/settings';

	const providerLabel = 'OpenRouter';
	let apiKey = $state('');
	let hasLoaded = $state(false);
	let showRemoveConfirm = $state(false);
	let removeButtonEl = $state<HTMLButtonElement | null>(null);
	let keyInputEl = $state<HTMLInputElement | null>(null);
	let confirmButtonEl = $state<HTMLButtonElement | null>(null);
	let cancelButtonEl = $state<HTMLButtonElement | null>(null);

	onMount(() => {
		void (async (): Promise<void> => {
			await privacyStore.loadStatus();
			hasLoaded = true;
		})();
	});

	function handleKeydown(event: KeyboardEvent): void {
		if (event.key === 'Escape') {
			event.preventDefault();
			showRemoveConfirm = false;
			removeButtonEl?.focus();
		}
	}

	async function handleSave(event: SubmitEvent): Promise<void> {
		event.preventDefault();
		const trimmed = apiKey.trim();
		if (!trimmed || privacyStore.loading) return;
		try {
			await privacyStore.setProviderKey(providerLabel, trimmed);
		} finally {
			apiKey = '';
		}
	}

	async function handleRemoveConfirm(): Promise<void> {
		await privacyStore.clearProviderKey(providerLabel);
		showRemoveConfirm = false;
		if (privacyStore.status === 'MISSING') {
			await tick();
			keyInputEl?.focus();
		} else {
			removeButtonEl?.focus();
		}
	}

	function handleRemoveCancel(): void {
		showRemoveConfirm = false;
		removeButtonEl?.focus();
	}
</script>

<div class="surface settings-surface" role="region" aria-label="Settings">
	<header class="surface-header">
		<h1 class="surface-title">Settings</h1>
	</header>

	<div class="surface-body">
		<section class="credentials-section" role="group" aria-labelledby="credentials-heading">
			<h2 id="credentials-heading" class="section-heading">API Credentials</h2>

			<div class="credential-card">
				<div class="provider-label">{providerLabel}</div>

				<div class="status-row" aria-live="polite">
					{#if privacyStore.loading}
						<span class="status-indicator status-indicator--loading" aria-hidden="true"></span>
						<span class="status-text status-text--muted">Checking…</span>
					{:else if privacyStore.status === 'CONFIGURED'}
						<span class="status-indicator status-indicator--configured" aria-hidden="true"></span>
						<span class="status-text">API key configured</span>
					{:else}
						<span class="status-indicator status-indicator--missing" aria-hidden="true"></span>
						<span class="status-text status-text--muted">No API key set</span>
					{/if}
				</div>

				{#if hasLoaded && privacyStore.status === 'MISSING' && !privacyStore.loading}
					<p class="empty-state">Paste your OpenRouter API key below to enable AI features.</p>

					<form class="key-form" onsubmit={handleSave}>
						<input
							bind:this={keyInputEl}
							type="password"
							autocomplete="off"
							aria-label="API key for OpenRouter"
							placeholder="Paste your API key"
							bind:value={apiKey}
						/>

						<div class="form-actions">
							<button
								class="btn-save"
								type="submit"
								disabled={privacyStore.loading || !apiKey.trim()}
								aria-label="Save API key for OpenRouter"
								aria-busy={privacyStore.loading}
							>
								Save API Key
							</button>
						</div>
					</form>
				{/if}

				{#if privacyStore.status === 'CONFIGURED'}
					<button
						bind:this={removeButtonEl}
						class="btn-clear"
						type="button"
						aria-label="Remove API key for OpenRouter"
						onclick={async () => {
							showRemoveConfirm = true;
							await tick();
							confirmButtonEl?.focus();
						}}
					>
						Remove key
					</button>
				{/if}

				{#if showRemoveConfirm}
					<div
						class="remove-confirm"
						role="alertdialog"
						aria-label="Confirm key removal"
						aria-modal="true"
						tabindex="0"
						onkeydown={handleKeydown}
					>
						<div class="remove-confirm__heading">Remove API key?</div>
						<p class="remove-confirm__body">
							This will clear the key from the OS keychain. You will need to re-enter it to use AI features.
						</p>
						<div class="remove-confirm__actions">
							<button
								bind:this={confirmButtonEl}
								class="confirm-remove"
								type="button"
								onclick={() => void handleRemoveConfirm()}
							>
								Confirm remove
							</button>
							<button
								bind:this={cancelButtonEl}
								class="cancel-remove"
								type="button"
								onclick={handleRemoveCancel}
							>
								Cancel
							</button>
						</div>
					</div>
				{/if}
			</div>

			{#if privacyStore.error}
				<p class="error-region" role="alert">{privacyStore.error}</p>
			{/if}
		</section>
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
		padding: 16px 24px;
		border-bottom: 1px solid #2a2a2a;
		flex: 0 0 auto;
	}

	.surface-title {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
		color: #e0e0e0;
	}

	.surface-body {
		flex: 1 1 auto;
		overflow: auto;
		padding: 24px;
	}

	.credentials-section {
		display: flex;
		flex-direction: column;
		gap: 24px;
		max-width: 480px;
	}

	.section-heading {
		margin: 0;
		font-size: 14px;
		font-weight: 600;
		line-height: 1.5;
		color: #e0e0e0;
	}

	.credential-card {
		display: flex;
		flex-direction: column;
		gap: 16px;
		background: #242424;
		border: 1px solid #2a2a2a;
		border-radius: 8px;
		padding: 16px;
	}

	.provider-label {
		font-size: 12px;
		font-weight: 400;
		color: #888;
	}

	.empty-state {
		margin: 0;
		font-size: 14px;
		font-weight: 400;
		color: #888;
	}

	.status-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.status-indicator {
		width: 8px;
		height: 8px;
		border-radius: 999px;
		flex: 0 0 auto;
	}

	.status-indicator--configured {
		background: #4a9eff;
	}

	.status-indicator--missing,
	.status-indicator--loading {
		background: #888;
	}

	.status-indicator--loading {
		animation: pulse 1.2s ease-in-out infinite;
	}

	.status-text {
		font-size: 14px;
		font-weight: 400;
		color: #e0e0e0;
	}

	.status-text--muted {
		color: #888;
	}

	.key-form {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.key-form input[type='password'] {
		height: 40px;
		padding: 0 12px;
		border-radius: 6px;
		border: 1px solid #2a2a2a;
		background: #1a1a1a;
		color: #e0e0e0;
		font-size: 14px;
	}

	.key-form input[type='password']::placeholder {
		color: #666;
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
	}

	.btn-save,
	.btn-clear,
	.confirm-remove,
	.cancel-remove,
	.key-form input[type='password'] {
		font-family: inherit;
	}

	.btn-save {
		padding: 8px 16px;
		border: none;
		border-radius: 6px;
		background: #4a9eff;
		color: #fff;
		font-size: 14px;
		font-weight: 500;
		cursor: pointer;
	}

	.btn-save:hover:not(:disabled) {
		background: #3a8eef;
	}

	.btn-save:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.btn-clear {
		align-self: flex-start;
		padding: 6px 0;
		border: none;
		background: transparent;
		color: #888;
		font-size: 14px;
		font-weight: 400;
		cursor: pointer;
	}

	.btn-clear:hover {
		color: #cc4444;
	}

	.remove-confirm {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px 16px;
		border-radius: 0 0 6px 6px;
		background: #2a1a1a;
		border-top: 1px solid #7a1e1e;
	}

	.remove-confirm__heading {
		font-size: 14px;
		font-weight: 400;
		color: #e0e0e0;
	}

	.remove-confirm__body {
		margin: 0;
		font-size: 14px;
		font-weight: 400;
		color: #888;
	}

	.remove-confirm__actions {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.confirm-remove {
		padding: 6px 12px;
		border-radius: 4px;
		border: 1px solid #7a1e1e;
		background: #2a1a1a;
		color: #cc4444;
		font-size: 14px;
		font-weight: 400;
		cursor: pointer;
	}

	.cancel-remove {
		padding: 6px 12px;
		border: none;
		background: transparent;
		color: #888;
		font-size: 14px;
		font-weight: 400;
		cursor: pointer;
	}

	.error-region {
		margin: 8px 0 0;
		color: #cc4444;
		font-size: 14px;
		font-weight: 400;
	}

	.btn-save:focus-visible,
	.btn-clear:focus-visible,
	.confirm-remove:focus-visible,
	.cancel-remove:focus-visible,
	.key-form input[type='password']:focus-visible {
		outline: 2px solid #4a9eff;
		outline-offset: 2px;
	}

	.key-form input[type='password']:focus-visible {
		border-color: #4a9eff;
		box-shadow: 0 0 0 2px rgba(74, 158, 255, 0.25);
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 0.4;
		}
		50% {
			opacity: 1;
		}
	}
</style>
