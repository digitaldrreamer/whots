import { apiJson, apiVoid } from './http';
import type { Notification } from './types';

export function listNotifications(): Promise<Notification[]> {
	return apiJson('/api/notifications');
}

export function unreadCount(): Promise<number> {
	return apiJson<{ count: number }>('/api/notifications/count').then((r) => r.count ?? 0);
}

export function markAllRead(): Promise<void> {
	return apiVoid('/api/notifications', { method: 'DELETE' });
}

export function markOneRead(id: string): Promise<void> {
	return apiVoid(`/api/notifications/${id}`, { method: 'PATCH' });
}
