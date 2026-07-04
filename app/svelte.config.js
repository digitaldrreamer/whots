import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// Node server so we can do server-side auth (httpOnly cookies + token
		// refresh in hooks.server) and talk to the Rust backend. SSR is disabled
		// per-route in +layout.ts because the game touches browser-only APIs.
		adapter: adapter()
	}
};

export default config;
