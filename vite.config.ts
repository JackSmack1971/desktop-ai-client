import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

// Tauri expects a specific dev server host/port so its built-in devtools proxy works.
// These values align with the Tauri v2 default dev URL expected by tauri.conf.json.
const TAURI_DEV_HOST = process.env.TAURI_DEV_HOST ?? 'localhost';

export default defineConfig({
	plugins: [sveltekit()],

	// Prevent Vite from obfuscating Rust panic messages in dev mode.
	clearScreen: false,

	server: {
		port: 1420,
		strictPort: true,
		host: TAURI_DEV_HOST,
		watch: {
			usePolling: false,
		},
	},

	build: {
		// Tauri uses Chromium on Windows, Firefox engine on Linux, WebKit on macOS.
		target:
			process.env.TAURI_ENV_PLATFORM === 'windows'
				? 'chrome105'
				: process.env.TAURI_ENV_PLATFORM === 'macos'
					? 'safari13'
					: 'firefox115',
		// Source maps in development; strip them in production to avoid exposing paths.
		sourcemap: !!process.env.TAURI_ENV_DEBUG,
		reportCompressedSize: true,
	},

	// Expose env vars to the frontend selectively. Do not expose secrets.
	envPrefix: ['VITE_', 'TAURI_ENV_'],
});
