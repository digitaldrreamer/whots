import { env } from '$env/dynamic/private';
import type { Cookies } from '@sveltejs/kit';

// Origin of the Rust backend. In containerized setups this is configured via
// INTERNAL_API_URL (e.g. http://server:3001); in local dev it defaults to localhost.
export const BACKEND_ORIGIN = env.INTERNAL_API_URL ?? 'http://localhost:3001';

// httpOnly cookie holding the rotating refresh token. Never exposed to the browser.
export const REFRESH_COOKIE = 'wh_refresh';
const REFRESH_MAX_AGE = 60 * 60 * 24 * 30; // 30 days

export interface BackendAuthResponse {
	user: unknown;
	access_token: string;
	refresh_token: string;
}

/** Call the Rust backend. `path` includes the leading `/api/...` or `/health`. */
export function backendFetch(path: string, init: RequestInit = {}): Promise<Response> {
	// A descriptive UA so local dev traverses Cloudflare (which challenges
	// unknown clients like Node's undici on /api/auth/*). In prod this call is
	// internal (http://server:3001) and never touches Cloudflare.
	return fetch(`${BACKEND_ORIGIN}${path}`, {
		...init,
		headers: { 'user-agent': 'whots-web/1.0', ...(init.headers as Record<string, string>) }
	});
}

export function setRefreshCookie(cookies: Cookies, token: string, secure: boolean): void {
	cookies.set(REFRESH_COOKIE, token, {
		path: '/',
		httpOnly: true,
		sameSite: 'lax',
		secure,
		maxAge: REFRESH_MAX_AGE
	});
}

export function clearRefreshCookie(cookies: Cookies): void {
	cookies.delete(REFRESH_COOKIE, { path: '/' });
}
