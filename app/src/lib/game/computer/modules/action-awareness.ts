import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

// Bias toward playing action cards over neutral ones.
// 5 (pick three) is valued most since it hurts the opponent hardest.
const ACTION_SCORES: Partial<Record<number, number>> = {
	5: 4,  // pick three
	2: 3,  // pick two
	14: 3, // general market
	1: 2,  // hold on
	8: 2   // suspension
};

export const actionAwareness: ScoringModule = (candidate: Candidate, _ctx: ModuleContext): number => {
	if (candidate.kind !== 'play-suit') return 0;
	return ACTION_SCORES[candidate.card.value] ?? 0;
};
