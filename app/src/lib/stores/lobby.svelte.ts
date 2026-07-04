import { session } from './session.svelte';
import { game } from '$lib/ui/game.svelte';
import { NotifySocket } from '$lib/api/notify-socket';
import * as social from '$lib/api/social';
import * as mm from '$lib/api/matchmaking';
import { acceptGame, declineGame, createGame } from '$lib/api/games';
import type { FriendRow, GameMode, Notification, PublicUser } from '$lib/api/types';

class LobbyStore {
	friends = $state<FriendRow[]>([]);
	requests = $state<PublicUser[]>([]);
	searchResults = $state<PublicUser[]>([]);
	inQueue = $state(false);
	queueMode = $state<GameMode | null>(null);
	pendingInvite = $state<{ gameId: string; from: string } | null>(null);
	toast = $state<string | null>(null);
	error = $state<string | null>(null);

	#notify: NotifySocket | null = null;

	/** Open the notify socket + load social data (call once authed). */
	async open(): Promise<void> {
		const token = session.accessToken;
		if (!token) return;
		this.#notify?.close();
		this.#notify = new NotifySocket(token, (n) => this.#onNotification(n));
		this.#notify.connect();
		await this.refresh();
	}

	close(): void {
		this.#notify?.close();
		this.#notify = null;
		this.reset();
	}

	async refresh(): Promise<void> {
		try {
			this.friends = await social.listFriends();
			this.requests = await social.incomingRequests();
		} catch {
			/* transient — leave prior state */
		}
	}

	#onNotification(n: Notification): void {
		const gid = typeof n.payload?.game_id === 'string' ? n.payload.game_id : undefined;
		const from = typeof n.payload?.from_username === 'string' ? n.payload.from_username : 'Someone';
		switch (n.kind) {
			case 'match_found':
				if (gid) {
					this.inQueue = false;
					this.queueMode = null;
					game.joinExisting(gid);
				}
				break;
			case 'game_invite':
				if (gid) this.pendingInvite = { gameId: gid, from };
				break;
			case 'game_accepted':
				this.#flash(`${from} accepted — game on!`);
				break;
			case 'game_declined':
				this.#flash(`${from} declined your invite`);
				break;
		}
	}

	// ── Matchmaking ────────────────────────────────────────────────────────────────
	async findMatch(mode: GameMode): Promise<void> {
		this.error = null;
		try {
			const res = await mm.joinQueue(mode);
			if (res.matched && res.game_id) {
				game.joinExisting(res.game_id);
			} else {
				this.inQueue = true;
				this.queueMode = mode;
			}
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Matchmaking failed.';
		}
	}

	async cancelMatch(): Promise<void> {
		await mm.leaveQueue().catch(() => undefined);
		this.inQueue = false;
		this.queueMode = null;
	}

	// ── Friends ────────────────────────────────────────────────────────────────────
	async search(q: string): Promise<void> {
		this.searchResults = q.trim().length >= 2 ? await social.searchUsers(q).catch(() => []) : [];
	}
	async addFriend(username: string): Promise<void> {
		try {
			await social.sendFriendRequest(username);
			this.#flash('Friend request sent');
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not send request.';
		}
	}
	async acceptRequest(username: string): Promise<void> {
		await social.acceptFriendRequest(username).catch(() => undefined);
		await this.refresh();
	}
	async declineRequest(username: string): Promise<void> {
		await social.declineFriendRequest(username).catch(() => undefined);
		await this.refresh();
	}
	async unfriend(username: string): Promise<void> {
		await social.removeFriend(username).catch(() => undefined);
		await this.refresh();
	}

	// ── Invites ────────────────────────────────────────────────────────────────────
	async inviteFriend(friend: FriendRow, mode: GameMode): Promise<void> {
		if (!session.user) return;
		this.error = null;
		try {
			const gameId = await createGame({
				mode,
				seats: [
					{ kind: 'human', user_id: session.user.id },
					{ kind: 'human', user_id: friend.id }
				]
			});
			game.joinExisting(gameId);
			this.#flash(`Invited ${friend.display_name}`);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not create game.';
		}
	}

	async acceptInvite(): Promise<void> {
		const inv = this.pendingInvite;
		if (!inv) return;
		this.pendingInvite = null;
		try {
			await acceptGame(inv.gameId);
			game.joinExisting(inv.gameId);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not join game.';
		}
	}

	async declineInvite(): Promise<void> {
		const inv = this.pendingInvite;
		if (!inv) return;
		this.pendingInvite = null;
		await declineGame(inv.gameId).catch(() => undefined);
	}

	#flash(msg: string): void {
		this.toast = msg;
		setTimeout(() => {
			if (this.toast === msg) this.toast = null;
		}, 2500);
	}

	reset(): void {
		this.friends = [];
		this.requests = [];
		this.searchResults = [];
		this.inQueue = false;
		this.queueMode = null;
		this.pendingInvite = null;
	}
}

export const lobby = new LobbyStore();
