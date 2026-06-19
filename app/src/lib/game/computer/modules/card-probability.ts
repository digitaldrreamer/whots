import { SUIT_VALUES } from '../../types.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

const MAX_PER_SHAPE = SUIT_VALUES.length; // 12

// Prefer landing the discard pile on a shape that has fewer cards remaining
// in the system (discard + own hand already subtracted in shapeRemaining).
// Fewer remaining → opponents less likely to hold that shape → harder for them to play.
export const cardProbability: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind !== 'play-suit') return 0;

	const remaining = ctx.shapeRemaining[candidate.card.shape];
	// Scale 0–2: more cards played = higher score
	return ((MAX_PER_SHAPE - remaining) / MAX_PER_SHAPE) * 4;
};
