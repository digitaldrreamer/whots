// Public (unauthenticated) auth endpoints. Called directly against the backend
// `/api/auth/*` — they don't set cookies, so they don't need the SvelteKit proxy.

async function post(path: string, body: unknown): Promise<Response> {
	return fetch(path, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
}

async function fail(res: Response): Promise<never> {
	const data = (await res.json().catch(() => ({}))) as { error?: string; message?: string };
	throw new Error(data?.error ?? data?.message ?? 'Request failed');
}

/** Always resolves (the server never reveals whether the email exists). */
export async function forgotPassword(email: string): Promise<void> {
	const res = await post('/api/auth/forgot-password', { email });
	if (!res.ok) await fail(res);
}

export async function resetPassword(token: string, newPassword: string): Promise<void> {
	const res = await post('/api/auth/reset-password', { token, new_password: newPassword });
	if (!res.ok) await fail(res);
}

export async function verifyEmail(token: string): Promise<void> {
	const res = await post('/api/auth/verify-email', { token });
	if (!res.ok) await fail(res);
}
