import { apiJson, apiVoid } from './http';
import type { FriendRow, PublicUser } from './types';

export function listFriends(): Promise<FriendRow[]> {
	return apiJson('/api/friends');
}

export function incomingRequests(): Promise<PublicUser[]> {
	return apiJson('/api/friends/requests');
}

export function sendFriendRequest(username: string): Promise<void> {
	return apiVoid(`/api/friends/request/${encodeURIComponent(username)}`, { method: 'POST' });
}

export function acceptFriendRequest(username: string): Promise<void> {
	return apiVoid(`/api/friends/request/${encodeURIComponent(username)}/accept`, { method: 'POST' });
}

export function declineFriendRequest(username: string): Promise<void> {
	return apiVoid(`/api/friends/request/${encodeURIComponent(username)}/decline`, { method: 'POST' });
}

export function removeFriend(username: string): Promise<void> {
	return apiVoid(`/api/friends/${encodeURIComponent(username)}`, { method: 'DELETE' });
}

/** Search users by username/display name (min 2 chars). */
export function searchUsers(q: string): Promise<PublicUser[]> {
	return apiJson(`/api/users/search?q=${encodeURIComponent(q)}`);
}
