import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import {
	backendFetch,
	clearRefreshCookie,
	REFRESH_COOKIE,
	setRefreshCookie,
	type BackendAuthResponse
} from '$lib/server/backend';

// Exchange the httpOnly refresh cookie for a fresh access token (and a rotated
// refresh cookie). Returns 401 with no session if the cookie is missing/expired.
export const POST: RequestHandler = async ({ cookies, url }) => {
	const refresh = cookies.get(REFRESH_COOKIE);
	if (!refresh) return json({ error: 'no session' }, { status: 401 });

	const res = await backendFetch('/api/auth/refresh', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ refresh_token: refresh })
	});

	if (!res.ok) {
		clearRefreshCookie(cookies);
		return json({ error: 'session expired' }, { status: 401 });
	}

	const auth = (await res.json()) as BackendAuthResponse;
	setRefreshCookie(cookies, auth.refresh_token, url.protocol === 'https:');
	return json({ user: auth.user, access_token: auth.access_token });
};
