import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

// Bias toward playing action cards over neutral ones.
// 5 (pick three) is valued most since it hurts the opponent hardest.
// Scores must dominate hand-thinning (max ~12) so action cards always win
// over non-action cards when action-awareness is active.
// Higher modules act as tiebreakers within this priority, not competitors.
const ACTION_SCORES: Partial<Record<number, number>> = {
	5: 30,  // pick three
	2: 25,  // pick two
	14: 20, // general market
	1: 15,  // hold on
	8: 15   // suspension
};

export const actionAwareness: ScoringModule = (candidate: Candidate, _ctx: ModuleContext): number => {
	if (candidate.kind !== 'play-suit') return 0;
	return ACTION_SCORES[candidate.card.value] ?? 0;
};
