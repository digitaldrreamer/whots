import { GameSocket, type SocketStatus } from '$lib/api/socket';
import { createGame } from '$lib/api/games';
import { canPlay, sameCard } from '$lib/api/rules';
import { session } from '$lib/stores/session.svelte';
import type {
	Card,
	Difficulty,
	GameMode,
	GameStateView,
	SeatSpec,
	SeatView,
	ServerEvent,
	Shape,
	TopCard
} from '$lib/api/types';
import { SHAPE_LABELS } from './theme.js';
import * as sound from './sound.js';

export type Screen = 'menu' | 'connecting' | 'playing' | 'result';
export type AnnounceTone = 'good' | 'bad' | 'wild' | 'skip' | 'market' | 'boss';
export type AnnounceData = { id: number; text: string; sub?: string; tone: AnnounceTone };
export type LogEntry = { id: number; who: 'you' | 'them' | 'system'; text: string };

export type GameConfig = {
	mode: GameMode;
	difficulty: Difficulty;
	opponents: number;
};

export type OpponentView = {
	index: number;
	name: string;
	handSize: number;
	isAi: boolean;
	isTee: boolean;
	isCurrent: boolean;
	owed: number; // General Market cards this opponent still owes the market
};

const OPPONENT_NAMES = ['Ada', 'Emeka', 'Ngozi', 'Bisi', 'Tunde'];

export const DIFFICULTY_META: { id: Difficulty; label: string; blurb: string }[] = [
	{ id: 'pikin', label: 'Pikin', blurb: 'Complete beginner — plays cards at random with no plan.' },
	{ id: 'smallz', label: 'Smallz', blurb: 'Still learning — mostly just tries to offload its hand.' },
	{ id: 'isabi_small', label: 'iSabiSmall', blurb: 'Getting the hang of it — starts using action cards.' },
	{ id: 'chief', label: 'Chief', blurb: 'Calculating — targets whoever is closest to winning.' },
	{ id: 'egbon', label: 'Ẹgbọn Àdúgbò', blurb: 'Reads the suits and calls Whot deliberately.' },
	{ id: 'jagaban', label: 'Jagaban', blurb: 'Anticipates your plays and sets traps. Ruthless.' }
];

const DIFFICULTY_LABELS: Record<Difficulty, string> = {
	pikin: 'Pikin',
	smallz: 'Smallz',
	isabi_small: 'iSabiSmall',
	chief: 'Chief',
	egbon: 'Ẹgbọn Àdúgbò',
	jagaban: 'Jagaban',
	tee_noble: 'Tee-Noble'
};

function topToCard(top: TopCard): Card {
	return top.kind === 'whot' ? { kind: 'whot' } : { kind: 'suit', shape: top.shape, value: top.value };
}

function cardName(card: Card): string {
	return card.kind === 'whot' ? 'Whot' : `${card.value} of ${SHAPE_LABELS[card.shape]}s`;
}

// value -> callout mapping for played action cards
const ACTION_LABEL: Record<number, string> = {
	1: 'HOLD ON',
	2: 'PICK TWO',
	5: 'PICK THREE',
	8: 'SUSPENDED',
	14: 'GENERAL MARKET'
};

export class GameController {
	screen = $state<Screen>('menu');
	config = $state<GameConfig>({ mode: 'stack', difficulty: 'chief', opponents: 1 });
	view = $state<GameStateView | null>(null);
	connection = $state<'idle' | 'connecting' | 'open' | 'closed' | 'error'>('idle');
	error = $state<string | null>(null);
	log = $state<LogEntry[]>([]);
	awaitingShape = $state(false);
	// A move has been sent and we're awaiting the server's echoed state. Used to
	// show "sending…" feedback and to lock input so a slow round-trip doesn't feel
	// frozen (and taps can't queue up).
	pending = $state(false);
	// Cards optimistically pulled from your hand the instant you play — so a laggy
	// round-trip doesn't leave your tap looking ignored. Reconciled on the server
	// echo, rolled back on error.
	#optRemoved = $state<Card[]>([]);
	// Cards tapped for a same-number stack (stack mode). One entry per card.
	selected = $state<Card[]>([]);

	// Animation / feedback signals (consumed by Board, Announce, FlightLayer, …)
	announce = $state<AnnounceData | null>(null);
	shakeId = $state(0);
	dealSeq = $state(0);
	winBurst = $state(0);
	teeIntro = $state(false);
	lastPlay = $state<{ id: number; seat: number; card: Card } | null>(null);
	lastDraw = $state<{ id: number; seat: number; count: number } | null>(null);
	// Transient AI "table talk" — a disappearing speech bubble over a seat.
	tableTalk = $state<{ id: number; seat: number; text: string } | null>(null);

	// Tee-Noble (wired in P6)
	isTeeGame = $state(false);
	teeChallenge = $state(false);

	#socket: GameSocket | null = null;
	#myUserId: string | null = null;
	#prev: GameStateView | null = null;
	#logId = 0;
	#annId = 0;
	#annTimer: ReturnType<typeof setTimeout> | null = null;
	#talkId = 0;
	#talkTimer: ReturnType<typeof setTimeout> | null = null;
	#pendingTimer: ReturnType<typeof setTimeout> | null = null;
	#flightId = 0;
	#winStreak = 0;

	// Inbound pacing. A laggy link buffers the server's already-spaced broadcasts
	// and TCP hands them over in one burst; applied as-is they'd all render in a
	// single frame (one snap, animations/sounds skipped or firing at once). Queue
	// them and drain one move at a time with a minimum readable gap. A single event
	// on an idle queue still applies immediately, so normal play stays instant.
	#queue: Exclude<ServerEvent, { type: 'error' }>[] = [];
	#draining = false;
	#drainTimer: ReturnType<typeof setTimeout> | null = null;

	// ── Derived ──────────────────────────────────────────────────────────────────

	get mySeatIndex(): number {
		const v = this.view;
		if (!v || !this.#myUserId) return -1;
		return v.seats.findIndex((s) => s.kind.kind === 'human' && s.kind.user_id === this.#myUserId);
	}

	get mySeat(): SeatView | null {
		const i = this.mySeatIndex;
		return i >= 0 ? (this.view?.seats[i] ?? null) : null;
	}

	get myHand(): Card[] {
		const hand = this.mySeat?.hand ?? [];
		if (this.#optRemoved.length === 0) return hand;
		// Drop the optimistically-played cards (shapes are unique within a play, so
		// an identity match removes exactly the right ones).
		return hand.filter((c) => !this.#optRemoved.some((r) => sameCard(c, r)));
	}

	get currentSeatIndex(): number {
		return this.view?.current_seat_index ?? -1;
	}

	get isMyTurn(): boolean {
		const v = this.view;
		return (
			v !== null &&
			v.phase === 'playing' &&
			this.currentSeatIndex === this.mySeatIndex &&
			!this.awaitingShape
		);
	}

	get busy(): boolean {
		return !this.isMyTurn;
	}

	/** True only when it's your turn AND the socket is live — the UI gates all
	 * play/draw input on this so clicks during a disconnect can't queue up and
	 * replay on reconnect. */
	get canAct(): boolean {
		return this.isMyTurn && this.connection === 'open' && !this.pending;
	}

	#markPending(): void {
		this.pending = true;
		if (this.#pendingTimer) clearTimeout(this.#pendingTimer);
		// Safety net: if no ack comes back in time the connection is likely bad and
		// our move is buffered in a stalled socket. Don't just unlock — that lets the
		// player tap again, buffering MORE sends that all replay on reconnect. Instead
		// force a fresh socket: it discards the buffered send and the server re-pushes
		// authoritative state, which clears `pending` and rolls back the optimistic play.
		this.#pendingTimer = setTimeout(() => {
			if (!this.pending) return;
			this.#socket?.reconnect();
		}, 5000);
	}

	#clearPending(): void {
		this.pending = false;
		this.#optRemoved = []; // reconcile: authoritative hand takes over (or roll back)
		if (this.#pendingTimer) {
			clearTimeout(this.#pendingTimer);
			this.#pendingTimer = null;
		}
	}

	get disconnected(): boolean {
		return this.screen === 'playing' && this.connection !== 'open';
	}

	get topCard(): TopCard | null {
		return this.view?.discard_top ?? null;
	}

	get mode(): GameMode {
		return this.view?.mode ?? this.config.mode;
	}

	/** Cards the player under the pending penalty would draw = count × per-card
	 * (a 2 = 2 cards each, a 5 = 3 cards each). 0 when no penalty is pending. */
	get pendingPick(): number {
		const p = this.view?.pending_effect;
		if (!p || p.kind !== 'pick') return 0;
		return p.count * (p.card === 5 ? 3 : 2);
	}

	/** The number (2 or 5) that started the pending penalty, or 0 if none. */
	get pendingCard(): number {
		const p = this.view?.pending_effect;
		return p && p.kind === 'pick' ? p.card : 0;
	}

	/** General Market cards *I* still have to go and draw myself (0 if none). */
	get myOwedDraws(): number {
		return this.mySeat?.owed_draws ?? 0;
	}

	/** Opponent seats in turn order starting after me (for seating them around the table). */
	get opponents(): OpponentView[] {
		const v = this.view;
		const me = this.mySeatIndex;
		if (!v || me < 0) return [];
		const out: OpponentView[] = [];
		for (let step = 1; step < v.seats.length; step++) {
			const idx = (me + step) % v.seats.length;
			const seat = v.seats[idx];
			out.push({
				index: idx,
				name: seat.name,
				handSize: seat.hand_size,
				isAi: seat.kind.kind === 'ai',
				isTee: seat.kind.kind === 'ai' && seat.kind.difficulty === 'tee_noble',
				isCurrent: idx === v.current_seat_index && v.phase === 'playing',
				owed: seat.owed_draws
			});
		}
		return out;
	}

	get thinkingName(): string | null {
		const v = this.view;
		if (!v || v.phase !== 'playing') return null;
		const cur = v.seats[v.current_seat_index];
		if (cur && cur.kind.kind === 'ai' && v.current_seat_index !== this.mySeatIndex) return cur.name;
		return null;
	}

	canPlayCard(card: Card): boolean {
		const v = this.view;
		if (!v || !this.isMyTurn) return false;
		// A General Market obligation must be settled first — you can only draw.
		if (this.myOwedDraws > 0) return false;
		return canPlay(card, v.discard_top, v.pending_effect, v.mode);
	}

	get playableCards(): Card[] {
		return this.myHand.filter((c) => this.canPlayCard(c));
	}

	/** The game opened on a Whot and it's my turn to declare its shape. */
	get mustCallShape(): boolean {
		return this.isMyTurn && this.view?.pending_effect?.kind === 'call_shape';
	}

	get mustDraw(): boolean {
		return (
			this.isMyTurn &&
			!this.mustCallShape &&
			(this.myOwedDraws > 0 || this.playableCards.length === 0)
		);
	}

	/** Whose turn it is / who the table is waiting on — surfaced in the UI so it's
	 * always clear who has to act (especially while waiting on a General Market draw). */
	get statusLine(): string {
		const v = this.view;
		if (!v || v.phase !== 'playing') return '';
		const cur = v.current_seat_index;
		const seat = v.seats[cur];
		const owed = seat?.owed_draws ?? 0;
		const callShape = v.pending_effect?.kind === 'call_shape';
		if (cur === this.mySeatIndex) {
			if (callShape) return 'Your turn — call a shape';
			if (owed > 0) return `Your turn — pick ${owed} from market`;
			if (this.pendingPick > 0) return `Your turn — counter or pick ${this.pendingPick}`;
			if (this.mustDraw) return 'Your turn — go to market';
			return 'Your turn';
		}
		const name = seat?.name ?? 'Opponent';
		if (callShape) return `Waiting for ${name} to choose a shape`;
		if (owed > 0) return `Waiting for ${name} to pick from market`;
		return `${name}'s turn`;
	}

	get winnerIndex(): number | null {
		return this.view?.winner_index ?? null;
	}

	get winnerIsMe(): boolean {
		return this.winnerIndex !== null && this.winnerIndex === this.mySeatIndex;
	}

	get winnerName(): string {
		const v = this.view;
		if (!v || v.winner_index === null) return '';
		return v.seats[v.winner_index]?.name ?? 'Winner';
	}

	get difficultyLabel(): string {
		return DIFFICULTY_LABELS[this.config.difficulty];
	}

	/** Chip label derived from the *actual* seats (not the stale menu difficulty),
	 * so a joined/room game shows the truth: "N players", the AI's difficulty, or
	 * "Mixed AI" when the room's AIs differ. */
	get tableLabel(): string {
		const v = this.view;
		if (!v) return this.difficultyLabel;
		const humans = v.seats.filter((s) => s.kind.kind === 'human').length;
		if (humans > 1) return `${humans} players`;
		const diffs = new Set(
			v.seats.filter((s) => s.kind.kind === 'ai').map((s) => (s.kind as { difficulty: Difficulty }).difficulty)
		);
		if (diffs.size === 0) return this.difficultyLabel;
		if (diffs.size === 1) return DIFFICULTY_LABELS[[...diffs][0]];
		return 'Mixed AI';
	}

	// ── Lifecycle ──────────────────────────────────────────────────────────────────

	async start(config: GameConfig): Promise<void> {
		this.config = { ...config };
		const seats: SeatSpec[] = [];
		for (let i = 0; i < config.opponents; i++) {
			seats.push({
				kind: 'ai',
				difficulty: config.difficulty,
				name: OPPONENT_NAMES[i] ?? `CPU ${i + 1}`
			});
		}
		await this.#launch(config.mode, seats, false);
	}

	/** Start the one-shot Tee-Noble duel — a single flawless server AI seat. */
	async startTee(): Promise<void> {
		this.teeChallenge = false;
		await this.#launch(this.config.mode, [{ kind: 'ai', difficulty: 'tee_noble', name: 'Tee-Noble' }], true);
	}

	/** Rematch: recreate the SAME table (same humans + AIs) from the finished
	 * game's seats — so a room doesn't collapse into a 1v1. Other humans get a
	 * fresh invite; AIs are re-seated at their difficulties. */
	async playAgain(): Promise<void> {
		if (this.isTeeGame) {
			await this.startTee();
			return;
		}
		const v = this.view;
		if (!v || v.seats.length < 2) {
			await this.start(this.config);
			return;
		}
		const seats: SeatSpec[] = v.seats.map((s) =>
			s.kind.kind === 'human'
				? { kind: 'human', user_id: s.kind.user_id }
				: { kind: 'ai', difficulty: s.kind.difficulty, name: s.name }
		);
		this.error = null;
		try {
			const gameId = await createGame({ mode: v.mode, seats });
			this.joinExisting(gameId);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not restart the game.';
			this.screen = 'menu';
		}
	}

	async #launch(mode: GameMode, aiSeats: SeatSpec[], isTee: boolean): Promise<void> {
		this.error = null;
		if (session.status !== 'authed' || !session.user) {
			this.error = 'Sign in first.';
			return;
		}
		this.#myUserId = session.user.id;
		this.isTeeGame = isTee;
		if (isTee) sound.preloadTeeLaugh();

		const seats: SeatSpec[] = [{ kind: 'human', user_id: session.user.id }, ...aiSeats];

		this.screen = 'connecting';
		this.connection = 'connecting';
		this.#resetFeedback();
		this.#pushLog(
			'system',
			isTee
				? 'Tee-Noble takes a seat across the table. No mercy.'
				: `New game — ${mode} mode vs ${this.difficultyLabel}.`
		);

		try {
			const gameId = await createGame({ mode, seats });
			this.#connect(gameId);
			if (isTee) {
				this.teeIntro = true;
				sound.playTeeLaugh();
				setTimeout(() => (this.teeIntro = false), 1900);
			}
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not start game.';
			this.screen = 'menu';
			this.connection = 'idle';
		}
	}

	/** Join an existing game (matchmaking match or an accepted invite). */
	joinExisting(gameId: string): void {
		if (session.status !== 'authed' || !session.user) {
			this.error = 'Sign in first.';
			return;
		}
		this.#myUserId = session.user.id;
		this.isTeeGame = false;
		this.screen = 'connecting';
		this.connection = 'connecting';
		this.#resetFeedback();
		this.#pushLog('system', 'Joining game…');
		this.#connect(gameId);
	}

	#connect(gameId: string): void {
		const token = session.accessToken;
		if (!token) {
			this.error = 'Not authenticated.';
			this.screen = 'menu';
			return;
		}
		this.#socket?.close();
		this.#prev = null;
		this.#socket = new GameSocket({
			gameId,
			token,
			onEvent: (ev) => this.#onEvent(ev),
			onStatus: (s) => this.#onStatus(s)
		});
		this.#socket.connect();
	}

	#onStatus(s: SocketStatus): void {
		this.connection = s === 'open' ? 'open' : s === 'error' ? 'error' : s === 'closed' ? 'closed' : 'connecting';
		// Any (re)connection means the server re-pushes authoritative state. Drop
		// any stale queued frames so we don't animate toward an out-of-date board
		// after a resync (e.g. the reconnect our bad-network fix triggers).
		if (s !== 'open') this.#flushQueue();
	}

	toMenu(): void {
		this.#socket?.close();
		this.#socket = null;
		this.view = null;
		this.#prev = null;
		this.screen = 'menu';
		this.connection = 'idle';
		this.isTeeGame = false;
		this.teeChallenge = false;
	}

	// ── Server events ───────────────────────────────────────────────────────────────

	#onEvent(ev: ServerEvent): void {
		// Errors surface immediately — they're about the local player's own move and
		// shouldn't wait behind paced opponent frames.
		if (ev.type === 'error') {
			this.#clearPending();
			this.error = ev.message;
			this.#pushLog('system', ev.message);
			return;
		}
		// Real-time signaling (deferred) must never sit behind the animation queue.
		if (ev.type === 'rtc_signal' || ev.type === 'chat_message') return; // handled elsewhere
		// Game frames render through the pace queue so a burst of buffered updates
		// plays out one move at a time instead of all snapping at once.
		this.#queue.push(ev);
		if (!this.#draining) this.#drain();
	}

	#drain(): void {
		const ev = this.#queue.shift();
		if (ev === undefined) {
			this.#draining = false;
			this.#drainTimer = null;
			return;
		}
		this.#draining = true;
		const remaining = this.#queue.length;
		// A deep backlog means a long stall — don't force the player to sit through
		// every frame animating. Fast-forward the stale ones silently; the newest
		// (drained once the queue shrinks) still animates normally.
		const collapse = remaining > 8;
		this.#applyEvent(ev, !collapse);
		if (this.#queue.length === 0) {
			this.#draining = false;
			this.#drainTimer = null;
			return;
		}
		// Minimum readable gap, shrinking as the queue deepens so we catch up.
		const gap = collapse ? 40 : remaining > 4 ? 140 : 480;
		this.#drainTimer = setTimeout(() => this.#drain(), gap);
	}

	#applyEvent(ev: Exclude<ServerEvent, { type: 'error' }>, animate: boolean): void {
		switch (ev.type) {
			case 'game_state':
				this.#applyView(ev.state, animate);
				break;
			case 'game_over':
				this.#onGameOver();
				break;
			case 'table_talk':
				if (animate) this.#showTableTalk(ev.seat, ev.text);
				break;
			// chat / rtc_signal handled elsewhere (deferred)
		}
	}

	#flushQueue(): void {
		this.#queue = [];
		this.#draining = false;
		if (this.#drainTimer) {
			clearTimeout(this.#drainTimer);
			this.#drainTimer = null;
		}
	}

	#applyView(next: GameStateView, animate = true): void {
		const prev = this.#prev;
		// New state arrived — the server processed our (or someone's) move, so the
		// in-flight lock and any stale card selection are done.
		this.#clearPending();
		this.selected = [];
		if (this.screen !== 'playing' && next.phase === 'playing') {
			this.screen = 'playing';
			this.dealSeq += 1;
			sound.playDeal(Math.min(next.seats.length * 2, 8));
		}
		this.view = next;

		// Feedback (flight animation, callouts, sounds) only for frames we're
		// actually animating; collapsed catch-up frames set state silently.
		if (animate) {
			if (prev) this.#deriveFeedback(prev, next);
			this.#checkLastCard(prev, next);
		}
		this.#prev = next;

		if (next.phase === 'finished') this.#onGameOver();
	}

	// Infer callouts/sounds/flight from the state transition.
	#deriveFeedback(prev: GameStateView, next: GameStateView): void {
		const mover = prev.current_seat_index;
		const me = this.mySeatIndex;
		const byMe = mover === me;
		const topChanged = JSON.stringify(prev.discard_top) !== JSON.stringify(next.discard_top);

		if (topChanged) {
			const card = topToCard(next.discard_top);
			this.#flightId += 1;
			this.lastPlay = { id: this.#flightId, seat: mover, card };

			if (next.discard_top.kind === 'whot') {
				const shape = next.discard_top.called_shape;
				this.#say('WHOT!', 'wild', `called ${SHAPE_LABELS[shape]}`);
				sound.play('whot');
				this.#pushLog(byMe ? 'you' : 'them', `${byMe ? 'You' : prev.seats[mover]?.name} played Whot — called ${SHAPE_LABELS[shape]}.`);
				return;
			}

			const value = next.discard_top.value;
			const stacking =
				next.mode === 'stack' &&
				next.pending_effect?.kind === 'pick' &&
				(value === 2 || value === 5);
			const comingToMe = next.current_seat_index === me && !byMe;
			const label = ACTION_LABEL[value];

			if (stacking) {
				this.#say(`STACK +${this.pendingPick}`, comingToMe ? 'bad' : 'good');
				sound.playStack(this.pendingPick);
			} else if (label) {
				const tone: AnnounceTone =
					value === 8 ? 'skip' : value === 14 ? 'market' : comingToMe ? 'bad' : 'good';
				this.#say(label, tone);
				sound.play(value === 1 ? 'holdon' : value === 2 ? 'pick2' : value === 5 ? 'pick3' : value === 8 ? 'skip' : 'market');
				if (comingToMe) this.#shake();
			} else {
				sound.play('play');
			}
			this.#pushLog(
				byMe ? 'you' : 'them',
				`${byMe ? 'You' : prev.seats[mover]?.name} played ${cardName(card)}.`
			);
		} else {
			// No card played -> a draw. The mover's hand grew.
			const grew = next.seats[mover] && prev.seats[mover] && next.seats[mover].hand_size > prev.seats[mover].hand_size;
			if (grew) {
				const drew = next.seats[mover].hand_size - prev.seats[mover].hand_size;
				this.#flightId += 1;
				this.lastDraw = { id: this.#flightId, seat: mover, count: drew };
				const wasPick = prev.pending_effect?.kind === 'pick';
				if (byMe && wasPick) {
					sound.play('youhit');
					this.#shake();
				} else {
					sound.play('draw');
				}
				this.#pushLog(
					byMe ? 'you' : 'them',
					`${byMe ? 'You' : prev.seats[mover]?.name} went to market${wasPick ? ` (${drew})` : ''}.`
				);
			}
		}
	}

	#checkLastCard(prev: GameStateView | null, next: GameStateView): void {
		if (next.phase !== 'playing') return;
		next.seats.forEach((seat, i) => {
			const before = prev?.seats[i]?.hand_size ?? 5;
			if (seat.hand_size === 1 && before !== 1) {
				const isYou = i === this.mySeatIndex;
				this.#say('LAST CARD', isYou ? 'good' : 'bad', isYou ? 'one to go!' : `${seat.name} is on 1`);
				sound.play('lastcard');
			}
		});
	}

	#onGameOver(): void {
		if (this.screen === 'result') return;
		const won = this.winnerIsMe;
		const wasTee = this.isTeeGame;
		const wasDuel = (this.view?.seats.length ?? 0) === 2;
		this.screen = 'result';
		this.awaitingShape = false;
		this.announce = null;

		if (won) {
			sound.play('win');
			this.winBurst += 1;
			this.#pushLog('system', 'You emptied your hand — you win! 🎉');
			if (wasTee) void session.refreshUser(); // pick up the freshly-earned badge
		} else {
			// When Tee-Noble beats you, his evil laugh is the send-off (not the
			// generic lose jingle).
			if (wasTee) sound.playTeeLaugh();
			else sound.play('lose');
			this.#pushLog('system', `${this.winnerName} wins.`);
		}

		// Tee-Noble stalks one-on-one duels — weighted to appear more on a streak.
		if (wasTee) {
			this.#winStreak = 0;
		} else if (won && wasDuel) {
			this.#winStreak += 1;
			const chance = Math.min(0.2 + this.#winStreak * 0.12, 0.65);
			if (Math.random() < chance) this.teeChallenge = true;
		} else if (!won) {
			this.#winStreak = 0;
		}
	}

	// ── Human actions ───────────────────────────────────────────────────────────────

	playSuit(card: Card): void {
		if (!this.canAct || card.kind !== 'suit') return;
		if (!this.canPlayCard(card)) return;
		this.#socket?.send({ type: 'play_card', action: { kind: 'suit', shape: card.shape, value: card.value } });
		this.#optRemoved = [card];
		this.#markPending();
	}

	// ── Stack selection (stack mode) ─────────────────────────────────────────────────

	get selectedValue(): number | null {
		const c = this.selected[0];
		return c && c.kind === 'suit' ? c.value : null;
	}

	isSelected(card: Card): boolean {
		return card.kind === 'suit' && this.selected.some((s) => s.kind === 'suit' && s.shape === card.shape && s.value === card.value);
	}

	/** Whether tapping this card does anything — a legal lead, or (stack mode) a
	 * same-number card that can be added to the current selection. */
	canTapCard(card: Card): boolean {
		if (!this.canAct) return false;
		if (card.kind === 'whot' || this.mode !== 'stack') return this.canPlayCard(card);
		if (this.selected.length === 0) return this.canPlayCard(card);
		return this.isSelected(card) || card.value === this.selectedValue;
	}

	get canConfirmSelection(): boolean {
		return this.canAct && this.selected.some((c) => this.canPlayCard(c));
	}

	/** Primary hand interaction. In no-stack a tap plays immediately; in stack mode
	 * a tap builds a same-number selection you confirm with playSelected(). */
	tapCard(card: Card): void {
		if (!this.canAct) return;
		if (card.kind === 'whot') {
			this.beginWhot();
			return;
		}
		if (this.mode !== 'stack') {
			this.playSuit(card);
			return;
		}
		// Stack mode: build/toggle the same-number selection.
		if (this.isSelected(card)) {
			this.selected = this.selected.filter(
				(s) => !(s.kind === 'suit' && card.kind === 'suit' && s.shape === card.shape && s.value === card.value)
			);
		} else if (this.selected.length === 0) {
			if (!this.canPlayCard(card)) return; // the lead card must be legal
			// Nothing to stack (you hold only one of this number) → play it now.
			const sameNumber = this.myHand.filter((c) => c.kind === 'suit' && c.value === card.value).length;
			if (sameNumber <= 1) {
				this.playSuit(card);
				return;
			}
			this.selected = [card];
		} else if (card.value === this.selectedValue) {
			this.selected = [...this.selected, card];
		} else {
			// different number → start a new selection with this card (if it can lead)
			this.selected = this.canPlayCard(card) ? [card] : [];
		}
	}

	playSelected(): void {
		if (!this.canConfirmSelection) return;
		const cards = this.selected.filter((c) => c.kind === 'suit') as Extract<Card, { kind: 'suit' }>[];
		if (cards.length === 0) return;
		const value = cards[0].value;
		const shapes = cards.map((c) => c.shape);
		this.selected = [];
		this.#optRemoved = cards;
		if (shapes.length === 1) {
			this.#socket?.send({ type: 'play_card', action: { kind: 'suit', shape: shapes[0], value } });
		} else {
			this.#socket?.send({ type: 'play_stack', value, shapes });
		}
		this.#markPending();
	}

	clearSelection(): void {
		this.selected = [];
	}

	beginWhot(): void {
		if (!this.canAct) return;
		if (!this.myHand.some((c) => c.kind === 'whot')) return;
		if (!this.canPlayCard({ kind: 'whot' })) return;
		this.awaitingShape = true;
	}

	chooseShape(shape: Shape): void {
		if (this.connection !== 'open') return;
		this.awaitingShape = false;
		// An opening Whot is declared with `call_shape` (no card leaves the hand);
		// a Whot played from hand uses `whot`.
		const opening = this.view?.pending_effect?.kind === 'call_shape';
		this.#socket?.send({
			type: 'play_card',
			action: opening
				? { kind: 'call_shape', called_shape: shape }
				: { kind: 'whot', called_shape: shape }
		});
		if (!opening) this.#optRemoved = [{ kind: 'whot' }]; // opening call plays no card
		this.#markPending();
	}

	cancelWhot(): void {
		this.awaitingShape = false;
	}

	draw(): void {
		if (!this.canAct) return;
		this.#socket?.send({ type: 'draw' });
		this.#markPending();
	}

	// ── Tee-Noble ────────────────────────────────────────────────────────────────────
	acceptTee(): void {
		void this.startTee();
	}
	declineTee(): void {
		this.teeChallenge = false;
		this.#winStreak = 0;
	}

	// ── Feedback helpers ─────────────────────────────────────────────────────────────

	#resetFeedback(): void {
		this.log = [];
		this.announce = null;
		this.lastPlay = null;
		this.lastDraw = null;
		this.tableTalk = null;
		this.awaitingShape = false;
		this.selected = [];
		this.error = null;
		this.#clearPending();
		this.#flushQueue();
	}

	#say(text: string, tone: AnnounceTone, sub?: string): void {
		this.#annId += 1;
		const id = this.#annId;
		this.announce = { id, text, tone, sub };
		if (this.#annTimer) clearTimeout(this.#annTimer);
		this.#annTimer = setTimeout(() => {
			if (this.#annId === id) this.announce = null;
		}, 1500);
	}

	#shake(): void {
		this.shakeId += 1;
	}

	#showTableTalk(seat: number, text: string): void {
		this.#talkId += 1;
		const id = this.#talkId;
		this.tableTalk = { id, seat, text };
		if (this.#talkTimer) clearTimeout(this.#talkTimer);
		this.#talkTimer = setTimeout(() => {
			if (this.#talkId === id) this.tableTalk = null;
		}, 3800);
	}

	#pushLog(who: LogEntry['who'], text: string): void {
		this.#logId += 1;
		this.log = [...this.log, { id: this.#logId, who, text }].slice(-40);
	}
}

export const game = new GameController();
