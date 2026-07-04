import { apiJson, apiVoid, jsonBody } from './http';
import type { Difficulty, GameMode, RoomView } from './types';

/** Create a pre-game room (you become the host). */
export function createRoom(mode: GameMode): Promise<{ room_id: string }> {
	return apiJson('/api/rooms', jsonBody({ mode }));
}

export function getRoom(id: string): Promise<RoomView> {
	return apiJson(`/api/rooms/${id}`);
}

/** Invite a friend (host only; friends-only, enforced server-side). */
export function inviteToRoom(id: string, userId: string): Promise<void> {
	return apiVoid(`/api/rooms/${id}/invite`, jsonBody({ user_id: userId }));
}

export function joinRoom(id: string): Promise<void> {
	return apiVoid(`/api/rooms/${id}/join`, { method: 'POST' });
}

export function leaveRoom(id: string): Promise<void> {
	return apiVoid(`/api/rooms/${id}/leave`, { method: 'POST' });
}

/** Add an AI seat of the given difficulty (host only). */
export function addRoomAi(id: string, difficulty: Difficulty): Promise<void> {
	return apiVoid(`/api/rooms/${id}/ai`, jsonBody({ difficulty }));
}

export function removeRoomAi(id: string, index: number): Promise<void> {
	return apiVoid(`/api/rooms/${id}/ai/${index}`, { method: 'DELETE' });
}

/** Start the game with everyone who has joined + the AI seats (host only). */
export function startRoom(id: string): Promise<{ game_id: string }> {
	return apiJson(`/api/rooms/${id}/start`, { method: 'POST' });
}
