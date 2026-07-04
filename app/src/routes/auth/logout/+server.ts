import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { backendFetch, clearRefreshCookie, REFRESH_COOKIE } from '$lib/server/backend';

// Revoke the refresh token server-side and drop the cookie. Best-effort — always
// clears the cookie even if the backend call fails.
export const POST: RequestHandler = async ({ cookies, request }) => {
	const refresh = cookies.get(REFRESH_COOKIE);
	const auth = request.headers.get('authorization');

	if (refresh && auth) {
		await backendFetch('/api/auth/logout', {
			method: 'DELETE',
			headers: { 'content-type': 'application/json', authorization: auth },
			body: JSON.stringify({ refresh_token: refresh })
		}).catch(() => undefined);
	}

	clearRefreshCookie(cookies);
	return json({ ok: true });
};
