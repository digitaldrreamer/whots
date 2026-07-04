import { session, ApiError } from '$lib/stores/session.svelte';
import type { CreateGameRequest, CreateGameResponse, GameStateView } from './types';

async function readError(res: Response): Promise<string> {
	const data = await res.json().catch(() => ({}));
	return (data as { error?: string; message?: string })?.error ?? data?.message ?? 'Request failed';
}

export async function createGame(req: CreateGameRequest): Promise<string> {
	const res = await session.apiFetch('/api/games', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(req)
	});
	if (!res.ok) throw new ApiError(res.status, await readError(res));
	return ((await res.json()) as CreateGameResponse).game_id;
}

export async function getGame(id: string): Promise<GameStateView> {
	const res = await session.apiFetch(`/api/games/${id}`);
	if (!res.ok) throw new ApiError(res.status, await readError(res));
	return (await res.json()) as GameStateView;
}

export async function cancelGame(id: string): Promise<void> {
	const res = await session.apiFetch(`/api/games/${id}`, { method: 'DELETE' });
	if (!res.ok && res.status !== 404) throw new ApiError(res.status, await readError(res));
}
