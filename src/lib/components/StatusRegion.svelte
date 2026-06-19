<script lang="ts">
	/**
	 * StatusRegion – screen-reader live region for shell status announcements.
	 *
	 * Announces surface switches, loading state, and backend errors to assistive
	 * technology without interrupting keyboard focus. Sighted users also see the
	 * status text in the status bar at the bottom of the shell.
	 *
	 * Accessibility:
	 *   - aria-live="polite" for surface switch announcements (non-interrupting).
	 *   - role="status" keeps the region discoverable in the AT tree.
	 *   - The visible label and the live-region text are kept in sync.
	 */

	interface Props {
		/** Short text describing the current shell state. */
		message?: string;
		/** Set to true when a backend operation is in progress. */
		loading?: boolean;
		/** Set to a non-null string when a backend error has occurred. */
		error?: string | null;
	}

	let { message = '', loading = false, error = null }: Props = $props();

	let announceText = $derived(
		error ? `Error: ${error}` : loading ? 'Loading…' : message,
	);
</script>

<!--
  Dual-region approach:
  - The visible bar carries the human-readable status label.
  - The aria-live region re-announces only when the derived text changes,
    which prevents stale AT announcements when Svelte re-renders unchanged props.
-->
<div
	class="status-region"
	role="status"
	aria-live="polite"
	aria-label="Shell status"
>
	<span class="status-region__text" class:status-region__text--error={!!error}>
		{announceText}
	</span>
</div>

<style>
	.status-region {
		display: flex;
		align-items: center;
		height: 24px;
		padding: 0 12px;
		background-color: #111;
		border-top: 1px solid #2a2a2a;
		font-size: 11px;
		color: #666;
		flex-shrink: 0;
		user-select: none;
	}

	.status-region__text {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status-region__text--error {
		color: #cc4444;
	}
</style>
