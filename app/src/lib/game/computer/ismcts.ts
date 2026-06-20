/**
 * Information Set Monte Carlo Tree Search (ISMCTS) move selector for tee-noble.
 *
 * Algorithm per move:
 *   For each simulation:
 *     1. Determinize: randomly redistribute the cards we can't see
 *        (opponent hands + stock pile) into plausible opponent hands,
 *        leaving our own hand unchanged.
 *     2. Apply each legal move to that world.
 *     3. Roll out the rest of the game with a fast heuristic for all players.
 *     4. Record win/loss per candidate.
 *   Pick the candidate with the highest win rate across all simulations.
 *
 * This naturally handles hidden information, stochastic draws, and N-player games.
 */

import type { Card, GameState } from '../types.js';
import type { PlayAction } from '../state.js';
import { drawCard, playCard } from '../state.js';
import { shuffle } from '../deck.js';
import { buildCandidates } from './context.js';
import { DEFAULT_PARAMS, selectMoveWithParams } from './params.js';
import type { Candidate } from './types.js';

// Rollout policies: tee-noble uses its best heuristic in simulations so its
// own future play is modelled accurately; opponents use chief (fast, realistic).
const ROLLOUT_PARAMS_SELF = DEFAULT_PARAMS['jagaban'];
const ROLLOUT_PARAMS_OPP  = DEFAULT_PARAMS['chief'];

const MAX_ROLLOUT_TURNS = 150;

// ── Helpers ────────────────────────────────────────────────────────────────────

function candidateToAction(c: Candidate): PlayAction | 'draw' {
	switch (c.kind) {
		case 'draw':
			return 'draw';
		case 'play-suit':
			return { kind: 'suit', card: c.card };
		case 'play-whot':
			return { kind: 'whot', calledShape: c.calledShape };
	}
}

/**
 * Build the pool of cards we cannot observe: every card that is neither
 * in our own hand nor on the discard pile. This equals stock + opponent hands.
 */
function buildUnseenPool(state: GameState, playerIndex: number): Card[] {
	const pool: Card[] = [...state.stockPile];
	for (let i = 0; i < state.players.length; i++) {
		if (i !== playerIndex) {
			pool.push(...(state.players[i]?.hand ?? []));
		}
	}
	return pool;
}

/**
 * Return a new GameState where opponent hands have been randomly filled
 * from the unseen pool, respecting each opponent's known hand size.
 * Our hand is unchanged. The rest of the pool becomes the new stock.
 */
function determinize(state: GameState, playerIndex: number): GameState {
	const pool = shuffle(buildUnseenPool(state, playerIndex));
	let cursor = 0;

	const newPlayers = state.players.map((p, i) => {
		if (i === playerIndex) return p;
		const newHand = pool.slice(cursor, cursor + p.hand.length);
		cursor += p.hand.length;
		return { ...p, hand: newHand };
	});

	return { ...state, players: newPlayers, stockPile: pool.slice(cursor) };
}

/**
 * Play a game to completion using the fast rollout policy for everyone.
 * Returns true if playerIndex wins.
 */
function rollout(state: GameState, playerIndex: number): boolean {
	let s = state;
	let turns = 0;

	while (s.phase === 'playing' && turns++ < MAX_ROLLOUT_TURNS) {
		const idx = s.currentPlayerIndex;
		try {
			const params = idx === playerIndex ? ROLLOUT_PARAMS_SELF : ROLLOUT_PARAMS_OPP;
			const action = selectMoveWithParams(s, idx, params);
			s = action === 'draw' ? drawCard(s, idx) : playCard(s, idx, action);
		} catch {
			break;
		}
	}

	return s.winner?.id === state.players[playerIndex]?.id;
}

// ── Public API ─────────────────────────────────────────────────────────────────

/**
 * Select a move using ISMCTS.
 *
 * @param numSimulations  Number of determinised worlds to sample. Higher = stronger
 *                        but slower. 200 works well in-game; use 30–50 for fast sims.
 */
export function selectMoveISMCTS(
	state: GameState,
	playerIndex: number,
	numSimulations = 200
): PlayAction | 'draw' {
	const candidates = buildCandidates(state, playerIndex);

	if (candidates.length === 0) return 'draw';
	if (candidates.length === 1) return candidateToAction(candidates[0]!);

	const wins = new Array<number>(candidates.length).fill(0);
	const plays = new Array<number>(candidates.length).fill(0);

	for (let sim = 0; sim < numSimulations; sim++) {
		// One determinised world per simulation — all candidates compete in the same world
		const det = determinize(state, playerIndex);

		for (let ci = 0; ci < candidates.length; ci++) {
			const action = candidateToAction(candidates[ci]!);
			plays[ci]!++;
			try {
				const afterMove =
					action === 'draw'
						? drawCard(det, playerIndex)
						: playCard(det, playerIndex, action);

				const won =
					afterMove.phase === 'finished'
						? afterMove.winner?.id === state.players[playerIndex]?.id
						: rollout(afterMove, playerIndex);

				if (won) wins[ci]!++;
			} catch {
				// Candidate illegal in this determinised world — counts as a loss
			}
		}
	}

	// Pick the candidate with the highest win rate; break ties by index
	let bestIdx = 0;
	let bestRate = -1;
	for (let ci = 0; ci < candidates.length; ci++) {
		const p = plays[ci] ?? 0;
		const rate = p > 0 ? (wins[ci] ?? 0) / p : 0;
		if (rate > bestRate) {
			bestRate = rate;
			bestIdx = ci;
		}
	}

	return candidateToAction(candidates[bestIdx]!);
}
