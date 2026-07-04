import { json, type Cookies } from '@sveltejs/kit';
import { backendFetch, setRefreshCookie, type BackendAuthResponse } from './backend';

/**
 * Proxy an auth call to the Rust backend, stash the rotating refresh token in an
 * httpOnly cookie, and return only { user, access_token } to the browser.
 */
export async function proxyAuth(
	path: string,
	payload: unknown,
	cookies: Cookies,
	secure: boolean
): Promise<Response> {
	const res = await backendFetch(path, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(payload)
	});
	const data = await res.json().catch(() => ({}));
	if (!res.ok) return json(data, { status: res.status });

	const auth = data as BackendAuthResponse;
	setRefreshCookie(cookies, auth.refresh_token, secure);
	return json({ user: auth.user, access_token: auth.access_token });
}
