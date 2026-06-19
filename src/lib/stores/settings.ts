/**
 * Backend-owned privacy settings store.
 *
 * Manages only credential status and key write/clear operations. The API key
 * value is never cached in this store or returned to the renderer.
 */

import { invoke } from '@tauri-apps/api/core';
import { normalizeIpcError } from '$lib/api/errors';

export type CredentialStatus = 'CONFIGURED' | 'MISSING';
export type ProviderId = 'OpenRouter';

function createPrivacyStore() {
	let status = $state<CredentialStatus>('MISSING');
	let loading = $state(false);
	let error = $state<string | null>(null);

	async function loadStatus(
		provider: ProviderId = 'OpenRouter',
	): Promise<void> {
		loading = true;
		error = null;
		try {
			status = await invoke<CredentialStatus>('privacy_get_credential_status', {
				provider,
			});
		} catch (e) {
			console.warn(
				'[privacy-store] status check failed:',
				normalizeIpcError(e),
			);
			status = 'MISSING';
			error = 'Could not check credential status. Try restarting the app.';
		} finally {
			loading = false;
		}
	}

	async function setProviderKey(
		provider: ProviderId,
		key: string,
	): Promise<void> {
		loading = true;
		error = null;
		try {
			await invoke<void>('privacy_set_provider_key', { provider, key });
			status = 'CONFIGURED';
		} catch (e) {
			console.warn('[privacy-store] save failed:', normalizeIpcError(e));
			error =
				'Failed to save key. Check your OS keychain access and try again.';
		} finally {
			loading = false;
		}
	}

	async function clearProviderKey(
		provider: ProviderId = 'OpenRouter',
	): Promise<void> {
		loading = true;
		error = null;
		try {
			await invoke<void>('privacy_clear_provider_key', { provider });
			status = 'MISSING';
		} catch (e) {
			console.warn('[privacy-store] clear failed:', normalizeIpcError(e));
			error =
				'Failed to remove key. Check your OS keychain access and try again.';
		} finally {
			loading = false;
		}
	}

	return {
		get status() {
			return status;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		loadStatus,
		setProviderKey,
		clearProviderKey,
	};
}

export const privacyStore = createPrivacyStore();
export { createPrivacyStore };
