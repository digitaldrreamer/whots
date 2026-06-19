import { SUIT_VALUES } from '../../types.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

const MAX_PER_SHAPE = SUIT_VALUES.length; // 12

// Prefer landing the discard pile on a shape that has fewer cards remaining
// in the system (discard + own hand already subtracted in shapeRemaining).
// Fewer remaining → opponents less likely to hold that shape → harder for them to play.
// Kept as a tiebreaker (max 2) so it nudges but never overrides action-awareness or thinning.
export const cardProbability: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind !== 'play-suit') return 0;

	const remaining = ctx.shapeRemaining[candidate.card.shape];
	const scale = 1 / Math.max(1, ctx.state.players.length - 1);
	return ((MAX_PER_SHAPE - remaining) / MAX_PER_SHAPE) * 2 * scale;
};
