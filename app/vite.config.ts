import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// In dev, proxy the backend (`/api` REST + WebSocket) to the local backend
// (or a remote one via BACKEND_URL) so `npm run dev` works out of the box.
// In production, the browser hits `/api` same-origin or reverse-proxy.
// The /auth/* endpoints are SvelteKit server routes and are NOT proxied.
const backendUrl = process.env.BACKEND_URL || 'http://localhost:3001';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/api': {
				target: backendUrl,
				changeOrigin: true,
				secure: false,
				ws: true
			}
		}
	}
});
