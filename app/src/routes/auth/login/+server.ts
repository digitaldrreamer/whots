import type { RequestHandler } from './$types';
import { proxyAuth } from '$lib/server/auth';

export const POST: RequestHandler = async ({ request, cookies, url }) => {
	const body = await request.json().catch(() => ({}));
	return proxyAuth(
		'/api/auth/login',
		{ identifier: body.identifier, password: body.password },
		cookies,
		url.protocol === 'https:'
	);
};
