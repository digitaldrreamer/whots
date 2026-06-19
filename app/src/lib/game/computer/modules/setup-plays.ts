import { isActionCard, isSuitCard, isWhotCard } from '../../guards.js';
import { getValidMoves } from '../../moves.js';
import type { TopCard } from '../../types.js';
import type { Candidate, ModuleContext, ScoringModule } from '../types.js';

// Prefer plays that leave us with more options on our next turn.
// Simulate the resulting top card, then count how many cards in our
// remaining hand would be playable against it.
export const setupPlays: ScoringModule = (candidate: Candidate, ctx: ModuleContext): number => {
	if (candidate.kind === 'draw') return 0;

	const player = ctx.state.players[ctx.playerIndex];
	if (player === undefined) return 0;

	// Simulate the top card that results from this play
	const simulatedTop: TopCard =
		candidate.kind === 'play-suit'
			? candidate.card
			: { kind: 'whot', value: 20, calledShape: candidate.calledShape };

	// Remove the played card from our hand to see what's left
	const remainingHand =
		candidate.kind === 'play-suit'
			? player.hand.filter(
					(c) =>
						!(
							isSuitCard(c) &&
							c.shape === candidate.card.shape &&
							c.value === candidate.card.value
						)
				)
			: player.hand.filter((c) => !isWhotCard(c));

	// Playing the last card — always the right move
	if (remainingHand.length === 0) return 50;

	// Count how many of our remaining cards could play against the simulated top
	const followUpMoves = getValidMoves(remainingHand, simulatedTop, null, ctx.state.mode);

	// In N-player games the top card will change N-1 times before we play again,
	// so setup quality is less predictable. Scale scores down accordingly.
	const scale = 1 / Math.max(1, ctx.state.players.length - 1);

	if (followUpMoves.length === 0) {
		// Action cards disrupt the opponent regardless of our follow-up position;
		// don't penalise them for landing on an awkward shape.
		if (candidate.kind === 'play-suit' && isActionCard(candidate.card)) return 0;
		// Non-action play that leaves us with nothing to follow — strong deterrent
		return -20 * scale;
	}

	// Cap at 6 so large hands don't overwhelm action-awareness in the score hierarchy
	return Math.min(6, followUpMoves.length * 1.5) * scale;
};
