import { apiJson } from './http';

/** Mint a one-use friend-invite token for the current user. */
export function createInvite(): Promise<{ token: string }> {
	return apiJson('/api/invites', { method: 'POST' });
}

/** Redeem an invite token — you and the creator become friends instantly. */
export function redeemInvite(token: string): Promise<{ username: string; display_name: string }> {
	return apiJson(`/api/invites/${encodeURIComponent(token)}/redeem`, { method: 'POST' });
}
