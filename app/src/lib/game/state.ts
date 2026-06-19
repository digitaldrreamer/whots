import type {
	ActionEffect,
	Card,
	GameMode,
	GameState,
	PendingEffect,
	Player,
	PlayerId,
	Shape,
	SuitCard,
	TopCard
} from './types.js';
import { createShuffledDeck, shuffle } from './deck.js';
import { isSuitCard, isWhotCard } from './guards.js';
import { getSuitCardEffect, getWhotEffect } from './effects.js';
import { canPlay, getValidMoves } from './moves.js';

const INITIAL_HAND_SIZE = 5;

export function createPlayerId(raw: string): PlayerId {
	return raw as PlayerId;
}

function extractStartingCard(pile: Card[]): { top: SuitCard; remaining: Card[] } {
	const remaining: Card[] = [];
	let top: SuitCard | undefined;

	for (const card of pile) {
		if (top === undefined && isSuitCard(card)) {
			top = card;
		} else {
			remaining.push(card);
		}
	}

	if (top === undefined) throw new Error('Deck contains no suit cards');
	return { top, remaining };
}

function reshuffleDiscard(state: GameState): GameState {
	if (state.stockPile.length > 0) return state;

	const [keep, ...toReshuffle] = [...state.discardPile].reverse();
	if (keep === undefined) return state;

	return { ...state, stockPile: shuffle(toReshuffle), discardPile: [keep] };
}

function advancePlayer(state: GameState, steps = 1): number {
	return (state.currentPlayerIndex + steps) % state.players.length;
}

// Compute the new pendingEffect after a card is played.
//
// - pick_two / pick_three accumulate into a pick total (stack mode lets the next player counter)
// - hold_on sets a skip, meaning A gets a follow-up turn and B is skipped after it
// - suspension, general_market, whot resolve fully in resolveNextTurn — no pending needed
// - null effect (non-action card) preserves existing pending (lets the hold_on skip survive
//   through A's follow-up play)
function computePending(
	effect: ActionEffect | null,
	current: PendingEffect | null
): PendingEffect | null {
	if (effect === null) return current;

	switch (effect.kind) {
		case 'pick_two':
			return { kind: 'pick', total: (current?.kind === 'pick' ? current.total : 0) + 2 };
		case 'pick_three':
			return { kind: 'pick', total: (current?.kind === 'pick' ? current.total : 0) + 3 };
		case 'hold_on':
			return { kind: 'skip' };
		case 'suspension':
		case 'general_market':
		case 'whot':
			return null;
	}
}

// --- Public API ---

export function createGame(players: Player[], mode: GameMode): GameState {
	const deck = createShuffledDeck();

	const totalToDeal = INITIAL_HAND_SIZE * players.length;
	const cardsToDeal = deck.slice(0, totalToDeal);
	const afterDeal = deck.slice(totalToDeal);

	const playersWithHands: Player[] = players.map((player, playerIndex) => ({
		...player,
		hand: cardsToDeal.filter((_, cardIndex) => cardIndex % players.length === playerIndex)
	}));

	const { top, remaining } = extractStartingCard(afterDeal);

	return {
		id: crypto.randomUUID(),
		mode,
		players: playersWithHands,
		stockPile: remaining,
		discardPile: [top],
		topCard: top,
		currentPlayerIndex: 0,
		phase: 'playing',
		pendingEffect: null,
		winner: null
	};
}

export type SuitCardPlay = { readonly kind: 'suit'; readonly card: SuitCard };
export type WhotCardPlay = { readonly kind: 'whot'; readonly calledShape: Shape };
export type PlayAction = SuitCardPlay | WhotCardPlay;

export function playCard(state: GameState, playerIndex: number, action: PlayAction): GameState {
	const player = state.players[playerIndex];
	if (player === undefined) throw new Error('Player not found');
	if (state.currentPlayerIndex !== playerIndex) throw new Error("Not this player's turn");
	if (state.phase !== 'playing') throw new Error('Game is not in progress');

	if (action.kind === 'suit') {
		const { card } = action;
		const cardIndex = player.hand.findIndex(
			(c) => isSuitCard(c) && c.shape === card.shape && c.value === card.value
		);
		if (cardIndex === -1) throw new Error('Card not in hand');
		if (!canPlay(card, state.topCard, state.pendingEffect, state.mode)) {
			throw new Error('Invalid move');
		}

		const newHand = player.hand.filter((_, i) => i !== cardIndex);
		const effect = getSuitCardEffect(card);
		const newPending = computePending(effect, state.pendingEffect);
		const updatedPlayers = state.players.map((p, i) =>
			i === playerIndex ? { ...p, hand: newHand } : p
		);
		const winner = newHand.length === 0 ? (updatedPlayers[playerIndex] ?? null) : null;

		return resolveNextTurn(
			{
				...state,
				players: updatedPlayers,
				discardPile: [...state.discardPile, card],
				topCard: card,
				pendingEffect: newPending,
				phase: winner ? 'finished' : 'playing',
				winner
			},
			effect
		);
	}

	// Whot play
	const whotIndex = player.hand.findIndex(isWhotCard);
	if (whotIndex === -1) throw new Error('No whot card in hand');
	if (!canPlay({ kind: 'whot', value: 20 }, state.topCard, state.pendingEffect, state.mode)) {
		throw new Error('Invalid move');
	}

	const newHand = player.hand.filter((_, i) => i !== whotIndex);
	const whotCard: Card = { kind: 'whot', value: 20 };
	const topCard: TopCard = { kind: 'whot', value: 20, calledShape: action.calledShape };
	const effect = getWhotEffect(action.calledShape);
	const updatedPlayers = state.players.map((p, i) =>
		i === playerIndex ? { ...p, hand: newHand } : p
	);
	const winner = newHand.length === 0 ? (updatedPlayers[playerIndex] ?? null) : null;

	return resolveNextTurn(
		{
			...state,
			players: updatedPlayers,
			discardPile: [...state.discardPile, whotCard],
			topCard,
			pendingEffect: null,
			phase: winner ? 'finished' : 'playing',
			winner
		},
		effect
	);
}

export function drawCard(state: GameState, playerIndex: number): GameState {
	if (state.currentPlayerIndex !== playerIndex) throw new Error("Not this player's turn");
	if (state.phase !== 'playing') throw new Error('Game is not in progress');

	const player = state.players[playerIndex];
	if (player === undefined) throw new Error('Player not found');

	// Pending pick: player couldn't counter — draw the full accumulated total
	if (state.pendingEffect?.kind === 'pick') {
		const count = state.pendingEffect.total;
		const refilled = reshuffleDiscard({ ...state, pendingEffect: null });
		const drawn = refilled.stockPile.slice(0, count);
		const newHand = [...player.hand, ...drawn];

		return {
			...refilled,
			players: refilled.players.map((p, i) => (i === playerIndex ? { ...p, hand: newHand } : p)),
			stockPile: refilled.stockPile.slice(count),
			currentPlayerIndex: advancePlayer(refilled)
		};
	}

	// Normal draw: only valid when the player has no playable cards
	const validMoves = getValidMoves(player.hand, state.topCard, state.pendingEffect, state.mode);
	if (validMoves.length > 0) throw new Error('Player has valid moves and must play');

	// A pending skip means this is the end of a hold_on chain — B still gets skipped after A draws
	const skipping = state.pendingEffect?.kind === 'skip';
	const refilled = reshuffleDiscard(state);

	if (refilled.stockPile.length === 0) {
		return {
			...refilled,
			pendingEffect: null,
			currentPlayerIndex: advancePlayer(refilled, skipping ? 2 : 1)
		};
	}

	const [drawn, ...rest] = refilled.stockPile;
	if (drawn === undefined) {
		return {
			...refilled,
			pendingEffect: null,
			currentPlayerIndex: advancePlayer(refilled, skipping ? 2 : 1)
		};
	}

	return {
		...refilled,
		players: refilled.players.map((p, i) =>
			i === playerIndex ? { ...p, hand: [...p.hand, drawn] } : p
		),
		stockPile: rest,
		pendingEffect: null,
		currentPlayerIndex: advancePlayer(refilled, skipping ? 2 : 1)
	};
}

function resolveNextTurn(state: GameState, effect: ActionEffect | null): GameState {
	if (state.winner) return state;

	switch (effect?.kind) {
		case 'hold_on':
			// A gets a follow-up turn. pendingEffect is already { kind: 'skip' } (set in playCard).
			// currentPlayerIndex stays at A. After A's follow-up the default branch will see the
			// skip and advance past B.
			return state;

		case 'suspension':
			// B is skipped entirely — no follow-up for current player
			return { ...state, pendingEffect: null, currentPlayerIndex: advancePlayer(state, 2) };

		case 'general_market': {
			const refilled = reshuffleDiscard(state);
			let stock = [...refilled.stockPile];

			const updatedPlayers = refilled.players.map((p, i) => {
				if (i === state.currentPlayerIndex) return p;
				const [card, ...rest] = stock;
				if (card === undefined) return p;
				stock = rest;
				return { ...p, hand: [...p.hand, card] };
			});

			return {
				...refilled,
				players: updatedPlayers,
				stockPile: stock,
				pendingEffect: null,
				currentPlayerIndex: advancePlayer(refilled)
			};
		}

		default: {
			// Covers: pick_two, pick_three, whot, non-action cards, and the end of a hold_on chain.
			// A pending skip here means A's follow-up just ended — advance past B.
			const skipping = state.pendingEffect?.kind === 'skip';
			return {
				...state,
				pendingEffect: skipping ? null : state.pendingEffect,
				currentPlayerIndex: advancePlayer(state, skipping ? 2 : 1)
			};
		}
	}
}
