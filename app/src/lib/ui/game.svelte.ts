import { GameSocket } from '$lib/api/socket';
import { createGame } from '$lib/api/games';
import { canPlay } from '$lib/api/rules';
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
};

const OPPONENT_NAMES = ['Ada', 'Emeka', 'Ngozi', 'Bisi', 'Tunde'];

export const DIFFICULTY_META: { id: Difficulty; label: string; blurb: string }[] = [
	{ id: 'pikin', label: 'Pikin', blurb: 'Pure beginner. Plays at random.' },
	{ id: 'smallz', label: 'Smallz', blurb: 'Learns to thin its hand.' },
	{ id: 'isabi_small', label: 'iSabiSmall', blurb: 'Starts using action cards.' },
	{ id: 'chief', label: 'Chief', blurb: 'Hunts the player with fewest cards.' },
	{ id: 'egbon', label: 'Ẹgbọn Àdúgbò', blurb: 'Reads suits and calls Whot smartly.' },
	{ id: 'jagaban', label: 'Jagaban', blurb: 'Anticipates and sets up plays. Ruthless.' }
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

	// Animation / feedback signals (consumed by Board, Announce, FlightLayer, …)
	announce = $state<AnnounceData | null>(null);
	shakeId = $state(0);
	dealSeq = $state(0);
	winBurst = $state(0);
	teeIntro = $state(false);
	lastPlay = $state<{ id: number; seat: number; card: Card } | null>(null);
	lastDraw = $state<{ id: number; seat: number } | null>(null);

	// Tee-Noble (wired in P6)
	isTeeGame = $state(false);
	teeChallenge = $state(false);

	#socket: GameSocket | null = null;
	#myUserId: string | null = null;
	#prev: GameStateView | null = null;
	#logId = 0;
	#annId = 0;
	#annTimer: ReturnType<typeof setTimeout> | null = null;
	#flightId = 0;

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
		return this.mySeat?.hand ?? [];
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

	get topCard(): TopCard | null {
		return this.view?.discard_top ?? null;
	}

	get mode(): GameMode {
		return this.view?.mode ?? this.config.mode;
	}

	get pendingPick(): number {
		const p = this.view?.pending_effect;
		return p && p.kind === 'pick' ? p.total : 0;
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
				isCurrent: idx === v.current_seat_index && v.phase === 'playing'
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
		return canPlay(card, v.discard_top, v.pending_effect, v.mode);
	}

	get playableCards(): Card[] {
		return this.myHand.filter((c) => this.canPlayCard(c));
	}

	get mustDraw(): boolean {
		return this.isMyTurn && this.playableCards.length === 0;
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

	// ── Lifecycle ──────────────────────────────────────────────────────────────────

	async start(config: GameConfig): Promise<void> {
		this.config = { ...config };
		this.error = null;
		if (session.status !== 'authed' || !session.user) {
			this.error = 'Sign in first.';
			return;
		}
		this.#myUserId = session.user.id;
		this.isTeeGame = false;

		const seats: SeatSpec[] = [{ kind: 'human', user_id: session.user.id }];
		for (let i = 0; i < config.opponents; i++) {
			seats.push({
				kind: 'ai',
				difficulty: config.difficulty,
				name: OPPONENT_NAMES[i] ?? `CPU ${i + 1}`
			});
		}

		this.screen = 'connecting';
		this.connection = 'connecting';
		this.#resetFeedback();
		this.#pushLog('system', `New game — ${config.mode} mode vs ${this.difficultyLabel}.`);

		try {
			const gameId = await createGame({ mode: config.mode, seats });
			this.#connect(gameId);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Could not start game.';
			this.screen = 'menu';
			this.connection = 'idle';
		}
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
			onStatus: (s) => (this.connection = s === 'open' ? 'open' : s === 'error' ? 'error' : s === 'closed' ? 'closed' : 'connecting')
		});
		this.#socket.connect();
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
		switch (ev.type) {
			case 'game_state':
				this.#applyView(ev.state);
				break;
			case 'game_over':
				this.#onGameOver();
				break;
			case 'error':
				this.error = ev.message;
				this.#pushLog('system', ev.message);
				break;
			// chat / rtc_signal handled elsewhere (deferred)
		}
	}

	#applyView(next: GameStateView): void {
		const prev = this.#prev;
		if (this.screen !== 'playing' && next.phase === 'playing') {
			this.screen = 'playing';
			this.dealSeq += 1;
			sound.playDeal(Math.min(next.seats.length * 2, 8));
		}
		this.view = next;

		if (prev) this.#deriveFeedback(prev, next);
		this.#checkLastCard(prev, next);
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
				this.lastDraw = { id: this.#flightId, seat: mover };
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
		this.screen = 'result';
		this.awaitingShape = false;
		this.announce = null;
		if (this.winnerIsMe) {
			sound.play('win');
			this.winBurst += 1;
			this.#pushLog('system', 'You emptied your hand — you win! 🎉');
		} else {
			sound.play('lose');
			this.#pushLog('system', `${this.winnerName} wins.`);
		}
	}

	// ── Human actions ───────────────────────────────────────────────────────────────

	playSuit(card: Card): void {
		if (!this.isMyTurn || card.kind !== 'suit') return;
		if (!this.canPlayCard(card)) return;
		this.#socket?.send({ type: 'play_card', action: { kind: 'suit', shape: card.shape, value: card.value } });
	}

	beginWhot(): void {
		if (!this.isMyTurn) return;
		if (!this.myHand.some((c) => c.kind === 'whot')) return;
		if (!this.canPlayCard({ kind: 'whot' })) return;
		this.awaitingShape = true;
	}

	chooseShape(shape: Shape): void {
		this.awaitingShape = false;
		this.#socket?.send({ type: 'play_card', action: { kind: 'whot', called_shape: shape } });
	}

	cancelWhot(): void {
		this.awaitingShape = false;
	}

	draw(): void {
		if (!this.isMyTurn) return;
		this.#socket?.send({ type: 'draw' });
	}

	// ── Tee-Noble (P6 stubs) ─────────────────────────────────────────────────────────
	acceptTee(): void {
		this.teeChallenge = false;
	}
	declineTee(): void {
		this.teeChallenge = false;
	}

	// ── Feedback helpers ─────────────────────────────────────────────────────────────

	#resetFeedback(): void {
		this.log = [];
		this.announce = null;
		this.lastPlay = null;
		this.lastDraw = null;
		this.awaitingShape = false;
		this.error = null;
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

	#pushLog(who: LogEntry['who'], text: string): void {
		this.#logId += 1;
		this.log = [...this.log, { id: this.#logId, who, text }].slice(-40);
	}
}

export const game = new GameController();
