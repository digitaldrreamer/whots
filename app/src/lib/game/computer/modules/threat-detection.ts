import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

// Cards that directly hinder the next player: pick-two, pick-three, hold-on, suspension
const HINDER_VALUES = new Set([1, 2, 5, 8]);
const THREAT_THRESHOLD = 3; // opponent with ≤ this many cards is a threat

// When the next player (the one who receives our action) is close to winning,
// heavily prefer playing 1, 2, 5, 8, or 14 to slow them down.
// Hold-on (1) is included: giving ourselves a follow-up turn when the opponent is about to win
// is as valuable as skipping them outright.
export const threatDetection: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind === 'draw') return 0;
	if (candidate.kind === 'play-whot') return 0;

	const { opponentHandSizes, state, playerIndex } = ctx;
	const playerCount = state.players.length;
	const nextIndex = (playerIndex + 1) % playerCount;

	const nextHandSize = opponentHandSizes[nextIndex] ?? Infinity;
	const minOpponentCards = Math.min(...opponentHandSizes.filter((s) => s !== -1));

	const threatIsNext = nextHandSize <= THREAT_THRESHOLD && nextHandSize === minOpponentCards;

	// Hinder cards aimed at next player
	if (threatIsNext && HINDER_VALUES.has(candidate.card.value)) return 20;
	// General market hurts everyone — strong bonus whenever anyone is close
	if (candidate.card.value === 14 && minOpponentCards <= THREAT_THRESHOLD) return 15;

	return 0;
};
