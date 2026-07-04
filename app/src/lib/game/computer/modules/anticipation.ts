import { SHAPES, SUIT_VALUES } from '../../types.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

const MAX_PER_SHAPE = SUIT_VALUES.length; // 12

// Prefer plays that leave the next player with fewer valid responses.
// Estimate P(next player can match by shape) from shapeRemaining and their hand size,
// then reward plays where that probability is low.
export const anticipation: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind === 'draw') return 0;
	// Whot-intelligence already chooses the hardest-to-match shape for whot plays
	if (candidate.kind === 'play-whot') return 0;

	const shape = candidate.card.shape;
	const shapeRemaining = ctx.shapeRemaining[shape];

	// Total unknown suit cards in the system (discard + own hand already removed)
	const totalRemaining = SHAPES.reduce((sum, s) => sum + ctx.shapeRemaining[s], 0);
	if (totalRemaining === 0) return 0;

	// Next player's hand size (the immediate opponent who must respond to our play)
	const nextPlayerIdx = (ctx.playerIndex + 1) % ctx.state.players.length;
	const nextHandSize = ctx.opponentHandSizes[nextPlayerIdx] ?? 5;
	if (nextHandSize <= 0) return 0;

	// P(next player holds at least one card of this shape) via independent-card approximation
	const pPerCard = shapeRemaining / totalRemaining;
	const pCanMatchByShape = 1 - Math.pow(1 - pPerCard, nextHandSize);

	// In N-player games the benefit of frustrating the immediate next player decays
	// because other players still get to respond before us. Scale accordingly.
	const scale = 1 / Math.max(1, ctx.state.players.length - 1);

	// Scale 0–8 in 2-player; proportionally smaller in multiplayer
	return (1 - pCanMatchByShape) * 8 * scale;
};
