import { session, ApiError } from '$lib/stores/session.svelte';

async function fail(res: Response): Promise<never> {
	const data = (await res.json().catch(() => ({}))) as { error?: string; message?: string };
	throw new ApiError(res.status, data?.error ?? data?.message ?? 'Request failed');
}

/** Authenticated JSON request against the backend (`/api/...`). */
export async function apiJson<T>(path: string, init?: RequestInit): Promise<T> {
	const res = await session.apiFetch(path, init);
	if (!res.ok) await fail(res);
	if (res.status === 204) return undefined as T;
	return (await res.json()) as T;
}

/** Authenticated request that returns no body. */
export async function apiVoid(path: string, init?: RequestInit): Promise<void> {
	const res = await session.apiFetch(path, init);
	if (!res.ok) await fail(res);
}

export function jsonBody(body: unknown): RequestInit {
	return { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) };
}
