import { SUIT_VALUES } from '../../types.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

const MAX_PER_SHAPE = SUIT_VALUES.length; // 12

// Prefer plays that leave the next player with fewer valid responses.
// We can't see their hand, so we use shapeRemaining as a proxy:
// if few cards of the resulting required shape remain in the system,
// the next player is less likely to have one.
export const anticipation: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind === 'draw') return 0;

	if (candidate.kind === 'play-suit') {
		const remaining = ctx.shapeRemaining[candidate.card.shape];
		// Low remaining → next player less likely to match by shape → harder for them
		return ((MAX_PER_SHAPE - remaining) / MAX_PER_SHAPE) * 4;
	}

	// For whot plays, anticipation defers to whot-intelligence which already handles shape choice
	return 0;
};
