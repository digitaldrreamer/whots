import {
	createGame,
	createPlayerId,
	drawCard,
	playCard,
	type PlayAction
} from '$lib/game/state.js';
import { getValidMoves } from '$lib/game/moves.js';
import { getSuitCardEffect } from '$lib/game/effects.js';
import { isWhotCard } from '$lib/game/guards.js';
import { selectMove, selectMoveTeeNoble } from '$lib/game/computer/index.js';
import {
	acceptChallenge,
	afterGame,
	createSession,
	declineChallenge,
	isChallengePending,
	resolveChallenge,
	type TeeNobleSession
} from '$lib/game/tee-noble.js';
import type {
	Card,
	Difficulty,
	GameMode,
	GameState,
	Player,
	Shape,
	SuitCard
} from '$lib/game/types.js';
import { SHAPE_LABELS } from './theme.js';
import * as sound from './sound.js';

export type Screen = 'menu' | 'playing' | 'result';

export type AnnounceTone = 'good' | 'bad' | 'wild' | 'skip' | 'market' | 'boss';
export type AnnounceData = { id: number; text: string; sub?: string; tone: AnnounceTone };

export type GameConfig = {
	mode: GameMode;
	difficulty: Difficulty;
	opponents: number;
};

export type LogEntry = {
	id: number;
	who: 'you' | 'them' | 'system';
	text: string;
};

const HUMAN = 0;
const AI_THINK_MS = 750;
const AI_STEP_MS = 550;

const OPPONENT_NAMES = ['Ada', 'Emeka', 'Ngozi'];

const DIFFICULTY_LABELS: Record<Difficulty, string> = {
	pikin: 'Pikin',
	smallz: 'Smallz',
	isabiSmall: 'iSabiSmall',
	chief: 'Chief',
	egbon: 'Ẹgbọn Àdúgbò',
	jagaban: 'Jagaban'
};

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function cardName(card: Card): string {
	if (card.kind === 'whot') return 'Whot';
	return `${card.value} of ${SHAPE_LABELS[card.shape]}s`;
}

function effectNote(card: SuitCard): string | null {
	const effect = getSuitCardEffect(card);
	switch (effect?.kind) {
		case 'hold_on':
			return 'Hold on!';
		case 'pick_two':
			return 'Pick two';
		case 'pick_three':
			return 'Pick three';
		case 'suspension':
			return 'Suspension';
		case 'general_market':
			return 'General market — everyone draws';
		default:
			return null;
	}
}

export class GameController {
	screen = $state<Screen>('menu');
	config = $state<GameConfig>({ mode: 'stack', difficulty: 'chief', opponents: 1 });
	state = $state<GameState | null>(null);
	log = $state<LogEntry[]>([]);
	busy = $state(false);
	thinkingName = $state<string | null>(null);
	awaitingShape = $state(false);

	// Tee-Noble
	tee = $state<TeeNobleSession>(createSession());
	teeChallenge = $state(false);
	isTeeGame = $state(false);

	// Gamification feedback
	announce = $state<AnnounceData | null>(null);
	shakeId = $state(0);
	winBurst = $state(0);

	#logId = 0;
	#annId = 0;
	#annTimer: ReturnType<typeof setTimeout> | null = null;
	#handSizes: number[] = [];

	// --- Derived helpers ---

	get human(): Player | null {
		return this.state?.players[HUMAN] ?? null;
	}

	get isHumanTurn(): boolean {
		const s = this.state;
		return (
			s !== null &&
			s.phase === 'playing' &&
			s.currentPlayerIndex === HUMAN &&
			!this.busy &&
			!this.awaitingShape
		);
	}

	get validHumanCards(): Card[] {
		const s = this.state;
		const h = this.human;
		if (!s || !h) return [];
		return getValidMoves(h.hand, s.topCard, s.pendingEffect, s.mode);
	}

	get canPlayCard(): (card: Card) => boolean {
		const valid = this.validHumanCards;
		return (card: Card) =>
			valid.some((v) =>
				v.kind === 'whot'
					? card.kind === 'whot'
					: card.kind === 'suit' && v.shape === card.shape && v.value === card.value
			);
	}

	get mustDraw(): boolean {
		const s = this.state;
		if (!s) return false;
		if (s.pendingEffect?.kind === 'pick') return this.validHumanCards.length === 0;
		return this.validHumanCards.length === 0;
	}

	get pendingPick(): number {
		return this.state?.pendingEffect?.kind === 'pick' ? this.state.pendingEffect.total : 0;
	}

	get winnerIsHuman(): boolean {
		return this.state?.winner?.id === this.human?.id;
	}

	// --- Setup ---

	#pushLog(who: LogEntry['who'], text: string) {
		this.#logId += 1;
		this.log = [...this.log, { id: this.#logId, who, text }].slice(-40);
	}

	// --- Feedback (banners, shake, sound) ---

	#say(text: string, tone: AnnounceTone, sub?: string) {
		this.#annId += 1;
		const id = this.#annId;
		this.announce = { id, text, tone, sub };
		if (this.#annTimer) clearTimeout(this.#annTimer);
		this.#annTimer = setTimeout(() => {
			if (this.#annId === id) this.announce = null;
		}, 1500);
	}

	#shake() {
		this.shakeId += 1;
	}

	// Called after a card is played — fires the matching callout + sound.
	#playFeedback(byIndex: number, action: PlayAction) {
		const s = this.state;
		if (!s) return;
		const players = s.players;
		const byHuman = byIndex === HUMAN;
		const nextIdx = (byIndex + 1) % players.length;
		const nextIsHuman = nextIdx === HUMAN;
		const nextName = players[nextIdx]?.name ?? 'next';

		if (action.kind === 'whot') {
			this.#say('WHOT!', 'wild', `called ${SHAPE_LABELS[action.calledShape]}`);
			sound.play('whot');
			return;
		}

		const effect = getSuitCardEffect(action.card);
		switch (effect?.kind) {
			case 'hold_on':
				this.#say('HOLD ON', 'good', byHuman ? 'go again' : `${players[byIndex]?.name} goes again`);
				sound.play('holdon');
				break;
			case 'pick_two':
				this.#say('PICK TWO', nextIsHuman && !byHuman ? 'bad' : 'good', `→ ${nextName}`);
				if (nextIsHuman && !byHuman) {
					sound.play('youhit');
					this.#shake();
				} else sound.play('pick2');
				break;
			case 'pick_three':
				this.#say('PICK THREE', nextIsHuman && !byHuman ? 'bad' : 'good', `→ ${nextName}`);
				if (nextIsHuman && !byHuman) {
					sound.play('youhit');
					this.#shake();
				} else sound.play('pick3');
				break;
			case 'suspension':
				this.#say('SUSPENDED', 'skip', `${nextName} skips`);
				sound.play('skip');
				if (nextIsHuman && !byHuman) this.#shake();
				break;
			case 'general_market':
				this.#say('GENERAL MARKET', 'market', 'everyone draws');
				sound.play('market');
				if (!byHuman) this.#shake();
				break;
			default:
				sound.play('play');
		}
	}

	// Announce when any player drops to their last card (once per transition).
	#checkLastCard() {
		const s = this.state;
		if (!s || s.phase !== 'playing') return;
		s.players.forEach((p, i) => {
			const prev = this.#handSizes[i] ?? 5;
			if (p.hand.length === 1 && prev !== 1) {
				const isYou = i === HUMAN;
				this.#say('LAST CARD', isYou ? 'good' : 'bad', isYou ? 'one to go!' : `${p.name} is on 1`);
				sound.play('lastcard');
			}
		});
		this.#handSizes = s.players.map((p) => p.hand.length);
	}

	#buildPlayers(kind: 'normal' | 'tee'): Player[] {
		const human: Player = {
			id: createPlayerId('human'),
			kind: 'human',
			name: 'You',
			hand: []
		};
		if (kind === 'tee') {
			return [
				human,
				{ id: createPlayerId('tee-noble'), kind: 'tee-noble', name: 'Tee-Noble', hand: [] }
			];
		}
		const opponents: Player[] = Array.from({ length: this.config.opponents }, (_, i) => ({
			id: createPlayerId(`cpu-${i}`),
			kind: 'computer' as const,
			name: OPPONENT_NAMES[i] ?? `CPU ${i + 1}`,
			difficulty: this.config.difficulty,
			hand: []
		}));
		return [human, ...opponents];
	}

	start(config: GameConfig) {
		this.config = { ...config };
		this.#startGame('normal');
	}

	#startGame(kind: 'normal' | 'tee') {
		this.isTeeGame = kind === 'tee';
		this.log = [];
		this.busy = false;
		this.thinkingName = null;
		this.awaitingShape = false;
		this.teeChallenge = false;
		const players = this.#buildPlayers(kind);
		this.state = createGame(players, this.config.mode);
		this.#handSizes = this.state.players.map((p) => p.hand.length);
		this.announce = null;
		this.screen = 'playing';
		if (kind === 'tee') {
			this.#pushLog('system', 'Tee-Noble takes a seat across the table. No mercy.');
			this.#say('TEE-NOBLE', 'boss', 'no mercy');
		} else {
			const diff = DIFFICULTY_LABELS[this.config.difficulty];
			this.#pushLog('system', `New game — ${this.config.mode} mode vs ${diff}. You start.`);
		}
	}

	toMenu() {
		this.screen = 'menu';
		this.state = null;
		this.teeChallenge = false;
		this.isTeeGame = false;
	}

	// --- Human actions ---

	playSuit(card: SuitCard) {
		if (!this.isHumanTurn) return;
		this.#applyHuman({ kind: 'suit', card }, card);
	}

	beginWhot() {
		if (!this.isHumanTurn) return;
		const h = this.human;
		if (!h || !h.hand.some(isWhotCard)) return;
		this.awaitingShape = true;
	}

	chooseShape(shape: Shape) {
		this.awaitingShape = false;
		this.#applyHuman({ kind: 'whot', calledShape: shape }, { kind: 'whot', value: 20 });
	}

	cancelWhot() {
		this.awaitingShape = false;
	}

	draw() {
		if (!this.isHumanTurn) return;
		const s = this.state;
		if (!s) return;
		const before = this.human?.hand.length ?? 0;
		const pick = this.pendingPick;
		try {
			this.state = drawCard(s, HUMAN);
		} catch {
			return;
		}
		const after = this.state.players[HUMAN]?.hand.length ?? 0;
		const drew = after - before;
		if (pick > 0) {
			this.#pushLog('you', `You went to market and picked ${drew} card${drew === 1 ? '' : 's'}.`);
			sound.play('youhit');
			this.#shake();
		} else {
			this.#pushLog('you', `You went to market (drew ${drew}).`);
			sound.play('draw');
		}
		this.#afterMove();
	}

	#applyHuman(action: PlayAction, played: Card) {
		const s = this.state;
		if (!s) return;
		try {
			this.state = playCard(s, HUMAN, action);
		} catch {
			this.#pushLog('system', 'That move is not allowed.');
			return;
		}
		let text = `You played ${cardName(played)}`;
		if (action.kind === 'whot') text += ` — called ${SHAPE_LABELS[action.calledShape]}`;
		else {
			const note = effectNote(action.card);
			if (note) text += ` — ${note}`;
		}
		this.#pushLog('you', text + '.');
		this.#playFeedback(HUMAN, action);
		this.#afterMove();
	}

	// --- Turn flow ---

	#afterMove() {
		const s = this.state;
		if (!s) return;
		this.#checkLastCard();
		if (s.phase === 'finished') {
			this.#endGame();
			return;
		}
		if (s.currentPlayerIndex !== HUMAN) {
			void this.#runAI();
		}
	}

	async #runAI() {
		this.busy = true;
		let firstStep = true;
		while (
			this.state &&
			this.state.phase === 'playing' &&
			this.state.currentPlayerIndex !== HUMAN
		) {
			const s = this.state;
			const idx = s.currentPlayerIndex;
			const player = s.players[idx];
			if (!player) break;
			this.thinkingName = player.name;
			await delay(firstStep ? AI_THINK_MS : AI_STEP_MS);
			firstStep = false;
			// Guard: state may have been reset (e.g. user left) while awaiting.
			if (!this.state || this.state !== s) return;

			const move: PlayAction | 'draw' =
				player.kind === 'tee-noble'
					? selectMoveTeeNoble(s, idx)
					: player.kind === 'computer'
						? selectMove(s, idx, player.difficulty)
						: 'draw';

			const before = player.hand.length;
			try {
				if (move === 'draw') {
					const pick = s.pendingEffect?.kind === 'pick' ? s.pendingEffect.total : 0;
					this.state = drawCard(s, idx);
					const after = this.state.players[idx]?.hand.length ?? before;
					const drew = after - before;
					this.#pushLog(
						'them',
						pick > 0
							? `${player.name} picked ${drew} card${drew === 1 ? '' : 's'}.`
							: `${player.name} went to market.`
					);
					sound.play('draw');
				} else if (move.kind === 'suit') {
					this.state = playCard(s, idx, move);
					const note = effectNote(move.card);
					this.#pushLog(
						'them',
						`${player.name} played ${cardName(move.card)}${note ? ` — ${note}` : ''}.`
					);
					this.#playFeedback(idx, move);
				} else {
					this.state = playCard(s, idx, move);
					this.#pushLog(
						'them',
						`${player.name} played Whot — called ${SHAPE_LABELS[move.calledShape]}.`
					);
					this.#playFeedback(idx, move);
				}
				this.#checkLastCard();
			} catch {
				// Engine rejected the AI move (shouldn't happen) — fall back to a draw
				// to keep the game from deadlocking.
				try {
					this.state = drawCard(s, idx);
				} catch {
					break;
				}
			}
		}
		this.busy = false;
		this.thinkingName = null;
		if (this.state?.phase === 'finished') this.#endGame();
	}

	#endGame() {
		this.busy = false;
		this.thinkingName = null;
		const won = this.winnerIsHuman;
		const s = this.state;
		this.#pushLog(
			'system',
			won ? 'You emptied your hand — you win! 🎉' : `${s?.winner?.name ?? 'Opponent'} wins.`
		);
		this.announce = null;
		if (won) {
			sound.play('win');
			this.winBurst += 1;
		} else {
			sound.play('lose');
		}

		if (this.isTeeGame) {
			this.tee = resolveChallenge(this.tee, won ? 'won' : 'lost');
		} else if (this.config.opponents === 1) {
			// Tee-Noble only stalks one-on-one duels.
			this.tee = afterGame(this.tee, won);
			if (isChallengePending(this.tee)) this.teeChallenge = true;
		}
		this.screen = 'result';
	}

	// --- Tee-Noble challenge ---

	acceptTee() {
		this.tee = acceptChallenge(this.tee);
		this.teeChallenge = false;
		this.#startGame('tee');
	}

	declineTee() {
		this.tee = declineChallenge(this.tee);
		this.teeChallenge = false;
	}

	get difficultyLabel(): string {
		return DIFFICULTY_LABELS[this.config.difficulty];
	}
}

export const DIFFICULTY_META: { id: Difficulty; label: string; blurb: string }[] = [
	{ id: 'pikin', label: 'Pikin', blurb: 'Pure beginner. Plays at random.' },
	{ id: 'smallz', label: 'Smallz', blurb: 'Learns to thin its hand.' },
	{ id: 'isabiSmall', label: 'iSabiSmall', blurb: 'Starts using action cards.' },
	{ id: 'chief', label: 'Chief', blurb: 'Hunts the player with fewest cards.' },
	{ id: 'egbon', label: 'Ẹgbọn Àdúgbò', blurb: 'Reads suits and calls Whot smartly.' },
	{ id: 'jagaban', label: 'Jagaban', blurb: 'Anticipates and sets up plays. Ruthless.' }
];

export const game = new GameController();
