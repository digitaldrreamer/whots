import { session } from './session.svelte';
import { game } from '$lib/ui/game.svelte';
import { NotifySocket } from '$lib/api/notify-socket';
import * as social from '$lib/api/social';
import * as mm from '$lib/api/matchmaking';
import { acceptGame, declineGame, createGame } from '$lib/api/games';
import * as rooms from '$lib/api/rooms';
import { createInvite } from '$lib/api/invites';
import type {
	Difficulty,
	FriendRow,
	GameMode,
	Notification,
	PublicUser,
	RoomView
} from '$lib/api/types';

class LobbyStore {
	friends = $state<FriendRow[]>([]);
	requests = $state<PublicUser[]>([]);
	inviteLink = $state<string | null>(null);
	inQueue = $state(false);
	queueMode = $state<GameMode | null>(null);
	pendingInvite = $state<{ gameId: string; from: string } | null>(null);
	// Multi-seat room lobby.
	room = $state<RoomView | null>(null);
	pendingRoomInvite = $state<{ roomId: string; from: string } | null>(null);
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
		const rid = typeof n.payload?.room_id === 'string' ? n.payload.room_id : undefined;
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
			case 'friend_added':
				this.#flash(`${from} is now your friend`);
				void this.refresh();
				break;
			case 'lobby_invite':
				if (rid) this.pendingRoomInvite = { roomId: rid, from };
				break;
			case 'lobby_update':
				if (rid && this.room?.id === rid) void this.#refreshRoom(rid);
				break;
			case 'lobby_closed':
				if (rid && this.room?.id === rid) {
					this.room = null;
					this.#flash('The host closed the room');
				}
				break;
			case 'game_start':
				if (gid) {
					this.room = null;
					game.joinExisting(gid);
				}
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
	/** Mint a one-use invite link to share privately. Discovery is invite-only. */
	async createInviteLink(): Promise<void> {
		this.error = null;
		try {
			const { token } = await createInvite();
			this.inviteLink = `${location.origin}/add/${token}`;
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not create invite link.';
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

	// ── Rooms (multi-seat lobby) ─────────────────────────────────────────────────────
	async createRoom(mode: GameMode): Promise<void> {
		this.error = null;
		try {
			const { room_id } = await rooms.createRoom(mode);
			await this.#refreshRoom(room_id);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not create room.';
		}
	}

	async #refreshRoom(id: string): Promise<void> {
		try {
			this.room = await rooms.getRoom(id);
		} catch {
			/* room may have closed — leave prior state, a lobby_closed will clear it */
		}
	}

	async inviteToRoom(friend: FriendRow): Promise<void> {
		if (!this.room) return;
		try {
			await rooms.inviteToRoom(this.room.id, friend.id);
			this.#flash(`Invited ${friend.display_name}`);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not invite.';
		}
	}

	async joinRoom(id: string): Promise<void> {
		this.pendingRoomInvite = null;
		this.error = null;
		try {
			await rooms.joinRoom(id);
			await this.#refreshRoom(id);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not join room.';
		}
	}

	declineRoomInvite(): void {
		this.pendingRoomInvite = null;
	}

	async addAi(difficulty: Difficulty): Promise<void> {
		if (!this.room) return;
		try {
			await rooms.addRoomAi(this.room.id, difficulty);
			await this.#refreshRoom(this.room.id);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not add AI.';
		}
	}

	async removeAi(index: number): Promise<void> {
		if (!this.room) return;
		try {
			await rooms.removeRoomAi(this.room.id, index);
			await this.#refreshRoom(this.room.id);
		} catch {
			/* ignore */
		}
	}

	async leaveRoom(): Promise<void> {
		const r = this.room;
		if (!r) return;
		this.room = null;
		await rooms.leaveRoom(r.id).catch(() => undefined);
	}

	async startRoom(): Promise<void> {
		const r = this.room;
		if (!r) return;
		this.error = null;
		try {
			const { game_id } = await rooms.startRoom(r.id);
			this.room = null;
			game.joinExisting(game_id);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not start game.';
		}
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
		this.inviteLink = null;
		this.inQueue = false;
		this.queueMode = null;
		this.pendingInvite = null;
		this.room = null;
		this.pendingRoomInvite = null;
	}
}

export const lobby = new LobbyStore();
