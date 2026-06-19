import { isSuitCard, isWhotCard } from '../../guards.js';
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

	// Count how many of our remaining cards could play against the simulated top
	const followUpMoves = getValidMoves(remainingHand, simulatedTop, null, ctx.state.mode);

	// Scale 0–3: each follow-up option adds a small bonus
	return followUpMoves.length * 0.5;
};
