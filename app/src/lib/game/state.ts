import type { Card, GameMode, GameState, Player, PlayerId, Shape, SuitCard, TopCard } from './types.js';
import { createShuffledDeck, shuffle } from './deck.js';
import { isSuitCard, isWhotCard } from './guards.js';
import { accumulate, getSuitCardEffect, getWhotEffect } from './effects.js';
import { canPlay, getValidMoves } from './moves.js';

const INITIAL_HAND_SIZE = 5;

// The single coercion boundary for PlayerId
export function createPlayerId(raw: string): PlayerId {
	return raw as PlayerId;
}

// Pulls the first suit card out of the pile to use as the opening card.
// Any whot cards encountered before it are moved to the back.
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

// Refills the stock pile from the discard pile when it runs out.
// The current top card stays on the discard pile.
function reshuffleDiscard(state: GameState): GameState {
	if (state.stockPile.length > 0) return state;

	const [keep, ...toReshuffle] = [...state.discardPile].reverse();
	if (keep === undefined) return state;

	return {
		...state,
		stockPile: shuffle(toReshuffle),
		discardPile: [keep]
	};
}

function advancePlayer(state: GameState, steps = 1): number {
	return (state.currentPlayerIndex + steps) % state.players.length;
}

// --- Public API ---

export function createGame(players: Player[], mode: GameMode): GameState {
	const deck = createShuffledDeck();

	const totalToDeal = INITIAL_HAND_SIZE * players.length;
	const cardsToDeal = deck.slice(0, totalToDeal);
	const afterDeal = deck.slice(totalToDeal);

	// Round-robin deal: player i gets cards at positions where cardIndex % playerCount === i
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

// A suit card play — calledShape is not relevant here
export type SuitCardPlay = {
	readonly kind: 'suit';
	readonly card: SuitCard;
};

// A whot card play always requires a calledShape
export type WhotCardPlay = {
	readonly kind: 'whot';
	readonly calledShape: Shape;
};

export type PlayAction = SuitCardPlay | WhotCardPlay;

export function playCard(state: GameState, playerIndex: number, action: PlayAction): GameState {
	const player = state.players[playerIndex];
	if (player === undefined) throw new Error('Player not found');
	if (state.currentPlayerIndex !== playerIndex) throw new Error("Not this player's turn");
	if (state.phase !== 'playing') throw new Error('Game is not in progress');

	// Locate the card in hand and validate the move
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
		const newPending = effect ? accumulate(effect, state.pendingEffect) : null;

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

	// Can only draw if no valid moves exist
	const validMoves = getValidMoves(player.hand, state.topCard, state.pendingEffect, state.mode);
	if (validMoves.length > 0) throw new Error('Player has valid moves and must play');

	const refilled = reshuffleDiscard(state);
	if (refilled.stockPile.length === 0) {
		// Stock exhausted even after reshuffle — skip turn
		return { ...refilled, currentPlayerIndex: advancePlayer(refilled) };
	}

	const [drawn, ...rest] = refilled.stockPile;
	if (drawn === undefined) return { ...refilled, currentPlayerIndex: advancePlayer(refilled) };

	const updatedPlayers = refilled.players.map((p, i) =>
		i === playerIndex ? { ...p, hand: [...p.hand, drawn] } : p
	);

	return {
		...refilled,
		players: updatedPlayers,
		stockPile: rest,
		currentPlayerIndex: advancePlayer(refilled)
	};
}

// Advance turn after a card is played, applying any immediate effect
function resolveNextTurn(state: GameState, effect: import('./types.js').ActionEffect | null): GameState {
	if (state.winner) return state;

	switch (effect?.kind) {
		case 'hold_on':
		case 'suspension':
			// Current player gets to play again (Card 1) or next player is skipped
			return {
				...state,
				currentPlayerIndex:
					effect.kind === 'hold_on'
						? state.currentPlayerIndex
						: advancePlayer(state, 2)
			};

		case 'general_market': {
			// Every other player draws one card
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
				currentPlayerIndex: advancePlayer(refilled)
			};
		}

		default:
			return { ...state, currentPlayerIndex: advancePlayer(state) };
	}
}
