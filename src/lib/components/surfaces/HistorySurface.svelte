<script lang="ts">
	/**
	 * HistorySurface – conversation history browser.
	 *
	 * Phase 1 scaffold: renders the history surface placeholder.
	 * Search, pagination, and conversation listing are implemented in later plans.
	 */
</script>

<div class="surface history-surface" role="region" aria-label="History">
	<header class="surface-header">
		<h1 class="surface-title">History</h1>
	</header>
	<div class="surface-body">
		<p class="surface-placeholder">
			History surface — conversation browser ships in a later plan.
		</p>
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
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: auto;
	}

	.surface-placeholder {
		color: #666;
		font-size: 14px;
		text-align: center;
	}
</style>
