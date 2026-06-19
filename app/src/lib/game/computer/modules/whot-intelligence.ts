import { SUIT_VALUES } from '../../types.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

const MAX_PER_SHAPE = SUIT_VALUES.length; // 12

// When playing a Whot, call the shape that opponents are least likely to hold.
// Fewer remaining in a shape → it's been mostly played → opponents probably don't have it.
export const whotIntelligence: ScoringModule = (
	candidate: Candidate,
	ctx: ModuleContext
): number => {
	if (candidate.kind !== 'play-whot') return 0;

	const remaining = ctx.shapeRemaining[candidate.calledShape];
	// Scale 0–8: calling a depleted shape is strongly preferred
	return ((MAX_PER_SHAPE - remaining) / MAX_PER_SHAPE) * 8;
};
