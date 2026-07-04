import type { RequestHandler } from './$types';
import { proxyAuth } from '$lib/server/auth';

// Finish a passkey login on the server so the rotating refresh token becomes an
// httpOnly cookie (same as /auth/login).
export const POST: RequestHandler = async ({ request, cookies, url }) => {
	const body = await request.json().catch(() => ({}));
	return proxyAuth(
		'/api/auth/passkey/login/finish',
		{ username: body.username, credential: body.credential },
		cookies,
		url.protocol === 'https:'
	);
};
