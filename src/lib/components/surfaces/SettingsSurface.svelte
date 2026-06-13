<script lang="ts">
	/**
	 * SettingsSurface – application settings panel.
	 *
	 * Phase 1 scaffold: renders the settings surface placeholder.
	 * Provider key management, privacy controls, and preferences ship in
	 * later plans. Provider credential commands use a separate settings
	 * capability and are never accessible from this component's context.
	 */
</script>

<div class="surface settings-surface" role="region" aria-label="Settings">
	<header class="surface-header">
		<h1 class="surface-title">Settings</h1>
	</header>
	<div class="surface-body">
		<p class="surface-placeholder">
			Settings surface — provider configuration ships in a later plan.
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
