import type { GameState } from '../types.js';
import type { PlayAction } from '../state.js';
import { buildCandidates, buildContext } from './context.js';
import type { Candidate, ModuleContext } from './types.js';
import { handThinning } from './modules/hand-thinning.js';
import { actionAwareness } from './modules/action-awareness.js';
import { threatDetection } from './modules/threat-detection.js';
import { cardProbability } from './modules/card-probability.js';
import { whotIntelligence } from './modules/whot-intelligence.js';
import { setupPlays } from './modules/setup-plays.js';
import { anticipation } from './modules/anticipation.js';

// ── Types ──────────────────────────────────────────────────────────────────────

/**
 * Continuous parameter vector for one difficulty level.
 *
 * Module weights (0 = disabled, 1 = baseline, >1 amplifies the signal).
 * Behavioural scalars are separate so they can be tuned independently.
 */
export type DifficultyParams = {
	// Module weights
	handThinning: number;
	actionAwareness: number;
	threatDetection: number;
	cardProbability: number;
	whotIntelligence: number;
	setupPlays: number;
	anticipation: number;

	// Behavioural tendencies
	noise: number; // σ of Gaussian jitter added to every candidate — high ≈ random
	bluffRate: number; // P(0–1) of deliberately choosing 2nd-best move (unpredictability)
};

/** All tunable keys in order — used by the optimizer to enumerate parameters. */
export const PARAM_KEYS: ReadonlyArray<keyof DifficultyParams> = [
	'handThinning',
	'actionAwareness',
	'threatDetection',
	'cardProbability',
	'whotIntelligence',
	'setupPlays',
	'anticipation',
	'noise',
	'bluffRate'
] as const;

/**
 * Valid range for each parameter.
 * The optimizer clamps values to [min, max] after each step.
 */
export const PARAM_BOUNDS: Readonly<Record<keyof DifficultyParams, [min: number, max: number]>> = {
	handThinning: [0, 4],
	actionAwareness: [0, 4],
	threatDetection: [0, 4],
	cardProbability: [0, 4],
	whotIntelligence: [0, 4],
	setupPlays: [0, 4],
	anticipation: [0, 4],
	noise: [0, 200],
	bluffRate: [0, 0.5]
};

// ── Default parameter sets (derived from current DIFFICULTY_MODULES config) ──

const BASE: DifficultyParams = {
	handThinning: 0,
	actionAwareness: 0,
	threatDetection: 0,
	cardProbability: 0,
	whotIntelligence: 0,
	setupPlays: 0,
	anticipation: 0,
	noise: 0.1,
	bluffRate: 0
};

/**
 * Default parameters for every level.
 * Mirrors the binary module-on/off design from DIFFICULTY_MODULES but
 * expressed as a continuous vector so the tuner can explore the space.
 */
export const DEFAULT_PARAMS: Record<
	'pikin' | 'smallz' | 'isabiSmall' | 'chief' | 'egbon' | 'jagaban' | 'tee-noble',
	DifficultyParams
> = {
	pikin: { ...BASE, noise: 100 }, // pure random — overwhelms all module signals
	smallz: { ...BASE, handThinning: 1 },
	isabiSmall: { ...BASE, handThinning: 1, actionAwareness: 1 },
	chief: { ...BASE, handThinning: 1, actionAwareness: 1, threatDetection: 1 },
	egbon: {
		...BASE,
		handThinning: 1,
		actionAwareness: 1,
		threatDetection: 1,
		cardProbability: 1,
		whotIntelligence: 1,
		setupPlays: 1
	},
	jagaban: {
		...BASE,
		handThinning: 1,
		actionAwareness: 1,
		threatDetection: 1,
		cardProbability: 1,
		whotIntelligence: 1,
		setupPlays: 1,
		anticipation: 1,
		noise: 0.05
	},
	'tee-noble': {
		...BASE,
		handThinning: 1,
		actionAwareness: 1,
		threatDetection: 1,
		cardProbability: 1,
		whotIntelligence: 1,
		setupPlays: 1,
		anticipation: 1,
		noise: 0.02
	}
};

// ── Scoring ────────────────────────────────────────────────────────────────────

function gaussianNoise(sigma: number): number {
	if (sigma <= 0) return 0;
	const u1 = Math.max(1e-10, Math.random());
	const u2 = Math.random();
	return sigma * Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
}

/** Score a single candidate using the continuous parameter weights. */
export function scoreCandidateWithParams(
	candidate: Candidate,
	ctx: ModuleContext,
	params: DifficultyParams
): number {
	const p = params;
	return (
		handThinning(candidate, ctx) * p.handThinning +
		actionAwareness(candidate, ctx) * p.actionAwareness +
		threatDetection(candidate, ctx) * p.threatDetection +
		cardProbability(candidate, ctx) * p.cardProbability +
		whotIntelligence(candidate, ctx) * p.whotIntelligence +
		setupPlays(candidate, ctx) * p.setupPlays +
		anticipation(candidate, ctx) * p.anticipation +
		gaussianNoise(p.noise)
	);
}

/** Choose an action using continuous params instead of the binary module system. */
export function selectMoveWithParams(
	state: GameState,
	playerIndex: number,
	params: DifficultyParams
): PlayAction | 'draw' {
	const candidates = buildCandidates(state, playerIndex);
	const ctx = buildContext(state, playerIndex, candidates);

	if (candidates.length === 0) return 'draw';

	// Score all candidates and sort descending
	const scored = candidates
		.map((c) => ({ c, score: scoreCandidateWithParams(c, ctx, params) }))
		.sort((a, b) => b.score - a.score);

	// Bluff: with probability bluffRate, pick 2nd best to be unpredictable
	let chosen = scored[0]!.c;
	if (params.bluffRate > 0 && scored.length > 1 && Math.random() < params.bluffRate) {
		chosen = scored[1]!.c;
	}

	switch (chosen.kind) {
		case 'draw':
			return 'draw';
		case 'play-suit':
			return { kind: 'suit', card: chosen.card };
		case 'play-whot':
			return { kind: 'whot', calledShape: chosen.calledShape };
	}
}

// ── Serialisation helpers ──────────────────────────────────────────────────────

export type ParamBundle = Record<string, DifficultyParams>;

export function clampParams(p: DifficultyParams): DifficultyParams {
	const result = { ...p };
	for (const key of PARAM_KEYS) {
		const [lo, hi] = PARAM_BOUNDS[key];
		result[key] = Math.max(lo, Math.min(hi, result[key]));
	}
	return result;
}
