import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

const PENALTY_VALUES = new Set([2, 5, 8]);
const THREAT_THRESHOLD = 3; // opponent with ≤ this many cards is a threat

// When the next player (the one who receives our penalty cards) is close to winning,
// heavily prefer playing 2, 5, 8, or 14 to slow them down.
export const threatDetection: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind === 'draw') return 0;

	const { opponentHandSizes, state, playerIndex } = ctx;
	const playerCount = state.players.length;
	const nextIndex = (playerIndex + 1) % playerCount;

	const nextHandSize = opponentHandSizes[nextIndex] ?? Infinity;
	const minOpponentCards = Math.min(
		...opponentHandSizes.filter((s) => s !== -1)
	);

	const threatIsNext = nextHandSize === minOpponentCards && nextHandSize <= THREAT_THRESHOLD;

	if (candidate.kind === 'play-suit') {
		// Penalty cards aimed at next player
		if (threatIsNext && PENALTY_VALUES.has(candidate.card.value)) return 6;
		// General market hurts everyone — good whenever any opponent is close
		if (candidate.card.value === 14 && minOpponentCards <= THREAT_THRESHOLD) return 4;
	}

	if (candidate.kind === 'play-whot' && threatIsNext) {
		// Calling a shape when the threat is next — bonus handled by whot-intelligence,
		// but add a base bonus here for awareness that next player is dangerous
		return 2;
	}

	return 0;
};
