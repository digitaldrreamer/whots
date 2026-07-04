import { isSuitCard } from '../../guards.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

// Prefer playing from the shape we hold the most cards of.
// Shedding the dominant shape fastest reduces the chance of getting stuck.
export const handThinning: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind !== 'play-suit') return 0;

	const player = ctx.state.players[ctx.playerIndex];
	if (player === undefined) return 0;

	const shapeCount = player.hand.filter(
		(c) => isSuitCard(c) && c.shape === candidate.card.shape
	).length;

	return shapeCount;
};
