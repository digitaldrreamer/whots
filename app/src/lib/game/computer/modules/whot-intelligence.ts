import { SUIT_VALUES } from '../../types.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

const MAX_PER_SHAPE = SUIT_VALUES.length; // 12

// When playing a Whot, call the shape that opponents are least likely to hold.
// Fewer remaining in a shape → it's been mostly played → opponents probably don't have it.
//
// Only activates when no suit card is available: playing Whot when a suit card exists wastes
// our most flexible card. When forced, choose the depleted shape; suit cards stay preferred.
export const whotIntelligence: ScoringModule = (
	candidate: Candidate,
	ctx: ModuleContext
): number => {
	if (candidate.kind !== 'play-whot') return 0;

	// If any suit play is available, do not bias toward Whot — let suit cards win
	const hasSuitOption = ctx.candidates.some((c) => c.kind === 'play-suit');
	if (hasSuitOption) return 0;

	const remaining = ctx.shapeRemaining[candidate.calledShape];
	const scale = 1 / Math.max(1, ctx.state.players.length - 1);
	return ((MAX_PER_SHAPE - remaining) / MAX_PER_SHAPE) * 15 * scale;
};
