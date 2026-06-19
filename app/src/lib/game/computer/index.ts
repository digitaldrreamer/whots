import { DIFFICULTY_MODULES } from '../types.js';
import type { Difficulty, GameState, ReasoningModule } from '../types.js';
import type { PlayAction } from '../state.js';
import { buildCandidates, buildContext } from './context.js';
import type { Candidate, ModuleContext, ScoringModule } from './types.js';
import { handThinning } from './modules/hand-thinning.js';
import { actionAwareness } from './modules/action-awareness.js';
import { threatDetection } from './modules/threat-detection.js';
import { cardProbability } from './modules/card-probability.js';
import { whotIntelligence } from './modules/whot-intelligence.js';
import { anticipation } from './modules/anticipation.js';
import { setupPlays } from './modules/setup-plays.js';

const MODULE_MAP: Record<ReasoningModule, ScoringModule> = {
	'hand-thinning': handThinning,
	'action-awareness': actionAwareness,
	'threat-detection': threatDetection,
	'card-probability': cardProbability,
	'whot-intelligence': whotIntelligence,
	anticipation: anticipation,
	'setup-plays': setupPlays
};

// All modules active — used by Tee-Noble
const ALL_MODULES = Object.values(MODULE_MAP);

function scoreCandidate(
	candidate: Candidate,
	ctx: ModuleContext,
	modules: readonly ScoringModule[]
): number {
	// Small noise prevents perfectly identical scores from being always resolved
	// in insertion order, making the computer feel slightly less mechanical
	const noise = Math.random() * 0.1;
	return modules.reduce((sum, mod) => sum + mod(candidate, ctx), noise);
}

function candidateToAction(candidate: Candidate): PlayAction | 'draw' {
	switch (candidate.kind) {
		case 'draw':
			return 'draw';
		case 'play-suit':
			return { kind: 'suit', card: candidate.card };
		case 'play-whot':
			return { kind: 'whot', calledShape: candidate.calledShape };
	}
}

function pickBest(
	candidates: Candidate[],
	ctx: ModuleContext,
	modules: readonly ScoringModule[]
): Candidate {
	let best = candidates[0];
	if (best === undefined) return { kind: 'draw' };

	let bestScore = scoreCandidate(best, ctx, modules);

	for (let i = 1; i < candidates.length; i++) {
		const candidate = candidates[i];
		if (candidate === undefined) continue;
		const score = scoreCandidate(candidate, ctx, modules);
		if (score > bestScore) {
			best = candidate;
			bestScore = score;
		}
	}

	return best;
}

function pickRandom(candidates: Candidate[]): Candidate {
	const index = Math.floor(Math.random() * candidates.length);
	return candidates[index] ?? { kind: 'draw' };
}

// --- Public API ---

export function selectMove(
	state: GameState,
	playerIndex: number,
	difficulty: Difficulty
): PlayAction | 'draw' {
	const candidates = buildCandidates(state, playerIndex);
	const ctx = buildContext(state, playerIndex, candidates);

	// Pikin has no modules — pure random
	if (difficulty === 'pikin') {
		return candidateToAction(pickRandom(candidates));
	}

	const activeModuleKeys = DIFFICULTY_MODULES[difficulty];
	const activeModules = activeModuleKeys.map((key) => MODULE_MAP[key]);
	const chosen = pickBest(candidates, ctx, activeModules);

	return candidateToAction(chosen);
}

// Tee-Noble always uses all modules — no difficulty tier
export function selectMoveTeeNoble(state: GameState, playerIndex: number): PlayAction | 'draw' {
	const candidates = buildCandidates(state, playerIndex);
	const ctx = buildContext(state, playerIndex, candidates);
	const chosen = pickBest(candidates, ctx, ALL_MODULES);
	return candidateToAction(chosen);
}
