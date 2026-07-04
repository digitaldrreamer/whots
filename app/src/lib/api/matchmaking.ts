import { apiJson, apiVoid, jsonBody } from './http';
import type { GameMode, MatchmakingJoinResponse, MatchmakingStatus } from './types';

/** Join the matchmaking queue for a mode. May match immediately (returns game_id). */
export function joinQueue(mode: GameMode): Promise<MatchmakingJoinResponse> {
	return apiJson('/api/matchmaking/join', jsonBody({ mode }));
}

export function leaveQueue(): Promise<void> {
	return apiVoid('/api/matchmaking/queue', { method: 'DELETE' });
}

export function queueStatus(): Promise<MatchmakingStatus> {
	return apiJson('/api/matchmaking/status');
}
