/**
 * Backend-owned surface preference store.
 *
 * This store is the single source of truth for which surface is currently
 * active. It reads and writes through typed IPC commands so the preference
 * is always persisted by the Rust backend rather than in browser storage.
 *
 * Privacy: this store never reads or writes localStorage, sessionStorage,
 * or any browser-local persistence. All state round-trips through the backend.
 */

import { invoke } from '@tauri-apps/api/core';
import { normalizeIpcError } from '$lib/api/errors';

/** Named surfaces the workspace shell can display. Must match Surface enum in app_state.rs. */
export type Surface = 'chat' | 'history' | 'settings' | 'artifacts';

/** Human-readable labels for each surface — used in status announcements. */
const SURFACE_LABELS: Record<Surface, string> = {
	chat: 'Chat',
	history: 'History',
	settings: 'Settings',
	artifacts: 'Artifacts',
};

function createSurfaceStore() {
	// Svelte 5 rune: mutable reactive state.
	let surface = $state<Surface>('chat');
	let loading = $state(false);
	let error = $state<string | null>(null);

	/**
	 * Hydrate the active surface from backend-owned SQLite persistence.
	 * Called once on layout mount. Falls back to 'chat' on any IPC failure
	 * so the shell always renders.
	 */
	async function hydrate(): Promise<void> {
		loading = true;
		error = null;
		try {
			const persisted = await invoke<Surface>('get_active_surface');
			surface = persisted;
		} catch (e) {
			// In development or before backend initializes, fall back gracefully.
			console.warn('[surface-store] Failed to load active surface from backend:', e);
			error = normalizeIpcError(e);
			// Default is already 'chat' from the $state initializer.
		} finally {
			loading = false;
		}
	}

	/**
	 * Switch the active surface and persist the new value to the backend.
	 * The optimistic update sets the local state immediately for responsive
	 * UI, then persists through IPC. On failure the state rolls back.
	 */
	async function setSurface(next: Surface): Promise<void> {
		const previous = surface;
		surface = next; // Optimistic update
		error = null;

		try {
			await invoke<void>('set_active_surface', { surface: next });
		} catch (e) {
			// Roll back the optimistic update if the backend call fails.
			surface = previous;
			error = normalizeIpcError(e);
			console.warn('[surface-store] Failed to persist active surface to backend:', e);
		}
	}

	return {
		get surface() { return surface; },
		get loading() { return loading; },
		get error() { return error; },
		/**
		 * Human-readable status message describing the current shell state.
		 * Used by StatusRegion to announce state changes to assistive technology.
		 */
		get statusMessage(): string {
			if (error) return `Error: ${error}`;
			if (loading) return 'Loading…';
			return `${SURFACE_LABELS[surface] ?? surface} surface active`;
		},
		hydrate,
		setSurface
	};
}

/**
 * Singleton store exported for use across the shell.
 *
 * Usage:
 *   import { surfaceStore } from '$lib/stores/surface';
 *   let active = $derived(surfaceStore.surface);
 */
export const surfaceStore = createSurfaceStore();
