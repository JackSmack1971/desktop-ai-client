import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// Use static adapter for Tauri: outputs files Tauri can serve as frontend dist.
		adapter: adapter({
			// Output directory aligns with tauri.conf.json frontendDist: "../dist".
			pages: '../dist',
			assets: '../dist',
			fallback: 'index.html',
			precompress: false,
			strict: true
		}),
		// Tauri does not use a real origin; mark it as trusted.
		csrf: { trustedOrigins: ['tauri://localhost', 'https://tauri.localhost', 'http://localhost:1420'] }
	}
};

export default config;
