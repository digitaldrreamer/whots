import { session, ApiError } from '$lib/stores/session.svelte';
import type {
	CreateGameRequest,
	CreateGameResponse,
	ExistingGameConflict,
	GameStateView,
	GameSummary
} from './types';

async function readError(res: Response): Promise<string> {
	const data = await res.json().catch(() => ({}));
	return (data as { error?: string; message?: string })?.error ?? data?.message ?? 'Request failed';
}

/**
 * Thrown when the server refuses to deal a second table against someone you're
 * already playing. Carries the running game so the caller can offer to resume it
 * — or retry with `force: true`.
 */
export class ExistingGameError extends ApiError {
	constructor(public existing: ExistingGameConflict) {
		super(409, 'you already have a game with this player');
		this.name = 'ExistingGameError';
	}
}

export async function createGame(req: CreateGameRequest): Promise<string> {
	const res = await session.apiFetch('/api/games', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(req)
	});
	if (res.status === 409) {
		const data = (await res.json().catch(() => ({}))) as Partial<ExistingGameConflict>;
		if (data.reason === 'existing_game') throw new ExistingGameError(data as ExistingGameConflict);
	}
	if (!res.ok) throw new ApiError(res.status, await readError(res));
	return ((await res.json()) as CreateGameResponse).game_id;
}

/** Games this user is a seat in, newest activity first. */
export async function myGames(status?: GameSummary['status']): Promise<GameSummary[]> {
	const qs = status ? `?status=${status}` : '';
	const res = await session.apiFetch(`/api/users/me/games${qs}`);
	if (!res.ok) throw new ApiError(res.status, await readError(res));
	return (await res.json()) as GameSummary[];
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

export async function acceptGame(id: string): Promise<void> {
	const res = await session.apiFetch(`/api/games/${id}/accept`, { method: 'POST' });
	if (!res.ok) throw new ApiError(res.status, await readError(res));
}

export async function declineGame(id: string): Promise<void> {
	const res = await session.apiFetch(`/api/games/${id}/decline`, { method: 'POST' });
	if (!res.ok) throw new ApiError(res.status, await readError(res));
}
