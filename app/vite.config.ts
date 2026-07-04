import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// In dev, proxy the backend (`/api` REST + WebSocket) to the deployed server so
// `npm run dev` works against real data. In production the browser hits `/api`
// same-origin and Traefik routes it to the Rust server. The /auth/* endpoints
// are SvelteKit server routes and are NOT proxied.
export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/api': {
				target: 'https://whots.drreamer.digital',
				changeOrigin: true,
				secure: true,
				ws: true
			}
		}
	}
});
