<script lang="ts">
	/**
	 * SearchBar — debounced search input for the History surface.
	 *
	 * Fires the onquery callback 300ms after the last keystroke (D-07).
	 * When the input is cleared, onquery receives an empty string which
	 * causes historyStore.search to fall back to history_list.
	 */

	interface Props {
		onquery: (_q: string) => void;
		disabled?: boolean;
	}

	let { onquery, disabled = false }: Props = $props();

	let text = $state('');
	let debounceTimer = $state<ReturnType<typeof setTimeout> | undefined>(
		undefined,
	);

	function handleInput(): void {
		clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			onquery(text.trim());
		}, 300); // D-07: 300ms debounce
	}

	// Cleanup: prevent post-unmount invocation (T-03-18)
	$effect(() => {
		return () => clearTimeout(debounceTimer);
	});
</script>

<form role="search" class="search-form" onsubmit={(e) => e.preventDefault()}>
	<input
		type="search"
		class="search-input"
		bind:value={text}
		oninput={handleInput}
		{disabled}
		placeholder="Search conversations…"
		aria-label="Search conversation history"
		aria-controls="conversation-list"
	/>
</form>

<style>
	.search-form {
		width: 100%;
	}

	.search-input {
		width: 100%;
		height: 40px;
		background: #1a1a1a;
		border: 1px solid #2a2a2a;
		border-radius: 6px;
		padding: 0 16px;
		color: #e0e0e0;
		font-size: 14px;
		font-family:
			system-ui,
			-apple-system,
			sans-serif;
		outline: none;
		box-sizing: border-box;
	}

	.search-input::placeholder {
		color: #666;
	}

	.search-input:focus {
		border-color: #4a9eff;
		box-shadow: 0 0 0 2px rgba(74, 158, 255, 0.25);
	}

	.search-input:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
