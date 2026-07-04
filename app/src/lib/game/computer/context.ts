import { SHAPES, SUIT_VALUES } from '../types.js';
import { isSuitCard, isWhotCard } from '../guards.js';
import { getValidMoves } from '../moves.js';
import type { GameState } from '../types.js';
import type { Candidate, ModuleContext } from './types.js';

const CARDS_PER_SHAPE = SUIT_VALUES.length; // 12 per shape in a full deck

// Expand valid cards into scored candidates.
// Whot cards are expanded into one candidate per shape (the choice is what shape to call).
export function buildCandidates(state: GameState, playerIndex: number): Candidate[] {
	const player = state.players[playerIndex];
	if (player === undefined) return [{ kind: 'draw' }];

	const valid = getValidMoves(player.hand, state.topCard, state.pendingEffect, state.mode);
	if (valid.length === 0) return [{ kind: 'draw' }];

	const candidates: Candidate[] = [];
	let whotExpanded = false;

	for (const card of valid) {
		if (isSuitCard(card)) {
			candidates.push({ kind: 'play-suit', card });
		} else if (isWhotCard(card) && !whotExpanded) {
			// All whot cards are identical — expand into one candidate per callable shape
			for (const shape of SHAPES) {
				candidates.push({ kind: 'play-whot', calledShape: shape });
			}
			whotExpanded = true;
		}
	}

	return candidates;
}

// Build the shared context that all modules operate on.
// Uses only public information: own hand + the discard pile.
export function buildContext(
	state: GameState,
	playerIndex: number,
	candidates: readonly Candidate[]
): ModuleContext {
	const player = state.players[playerIndex];

	// Count shapes accounted for by own hand and the discard pile
	const accounted: Record<string, number> = {};
	for (const shape of SHAPES) accounted[shape] = 0;

	for (const card of state.discardPile) {
		if (isSuitCard(card)) accounted[card.shape]++;
	}
	if (player !== undefined) {
		for (const card of player.hand) {
			if (isSuitCard(card)) accounted[card.shape]++;
		}
	}

	const shapeRemaining = SHAPES.reduce(
		(acc, shape) => {
			acc[shape] = Math.max(0, CARDS_PER_SHAPE - (accounted[shape] ?? 0));
			return acc;
		},
		{} as Record<(typeof SHAPES)[number], number>
	);

	const opponentHandSizes = state.players.map((p, i) =>
		i === playerIndex ? -1 : p.hand.length
	);

	return { state, playerIndex, candidates, opponentHandSizes, shapeRemaining };
}
