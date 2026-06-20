/**
 * ┌────────────────────────────────────────────────────────────────────────────┐
 * │  WHOTS AI DIFFICULTY TUNER                                                 │
 * │  Coordinate-descent optimizer for DifficultyParams                         │
 * │                                                                            │
 * │  Goal: find params so every higher level beats every lower level by at     │
 * │  least 60% in both 1v1 and 4-player group games.                          │
 * │                                                                            │
 * │  Usage:                                                                    │
 * │    npx tsx scripts/tune.ts                 # run with defaults             │
 * │    npx tsx scripts/tune.ts --games 2000    # more games per eval           │
 * │    npx tsx scripts/tune.ts --continuous    # restart on convergence (hours)│
 * │    npx tsx scripts/tune.ts --resume        # continue from checkpoint      │
 * │    npx tsx scripts/tune.ts --verify        # verify only, no optimization  │
 * │                                                                            │
 * │  The script saves a checkpoint after every improvement to                  │
 * │    scripts/params/best.json                                                │
 * │  Ctrl+C at any time — progress is not lost.                               │
 * └────────────────────────────────────────────────────────────────────────────┘
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { createGame, createPlayerId, drawCard, playCard } from '../src/lib/game/state.js';
import type { GameMode, PlayerId, Player } from '../src/lib/game/types.js';
import {
	DEFAULT_PARAMS,
	PARAM_KEYS,
	PARAM_BOUNDS,
	clampParams,
	selectMoveWithParams
} from '../src/lib/game/computer/params.js';
import type { DifficultyParams } from '../src/lib/game/computer/params.js';

// ── Configuration ──────────────────────────────────────────────────────────────

const ARGS = parseArgs();

const GAMES_PER_EVAL = ARGS.games ?? 500; // games per head-to-head matchup
const MAX_SWEEPS = ARGS.sweeps ?? 50; // max full sweeps before stopping
const INITIAL_STEP = ARGS.step ?? 0.15; // starting step size for param changes
const MIN_STEP = 0.005; // stop shrinking below this
const STEP_DECAY = 0.6; // shrink factor when no improvement in a sweep
const MODE: GameMode = 'stack';
const MAX_TURNS = 600;
const CHECKPOINT_PATH = 'scripts/params/best.json';

// Wider-margin / multiplayer targets
const TARGET_MARGIN = 0.60; // minimum win rate for a pair to count as "dominant"
const MULTI_WEIGHT = 0.35;  // fraction of global score from multiplayer games
const MULTI_GAMES_RATIO = 0.4; // multiplayer games = GAMES_PER_EVAL * this
const PERTURB_STRENGTH = 0.5;  // jitter magnitude on continuous-mode restarts

// The difficulty ladder, lowest to highest
const LADDER = [
	'pikin',
	'smallz',
	'isabiSmall',
	'chief',
	'egbon',
	'jagaban',
	'tee-noble'
] as const;
type Level = (typeof LADDER)[number];

// pikin and tee-noble are fixed — only these 5 are tunable
const TUNABLE: Level[] = ['smallz', 'isabiSmall', 'chief', 'egbon', 'jagaban'];

// ── Argument parsing ───────────────────────────────────────────────────────────

function parseArgs(): {
	games?: number;
	sweeps?: number;
	step?: number;
	resume: boolean;
	verify: boolean;
	continuous: boolean;
} {
	const args = process.argv.slice(2);
	const result: ReturnType<typeof parseArgs> = { resume: false, verify: false, continuous: false };
	for (let i = 0; i < args.length; i++) {
		const a = args[i];
		if (a === '--resume') result.resume = true;
		else if (a === '--verify') result.verify = true;
		else if (a === '--continuous') result.continuous = true;
		else if (a === '--games' && args[i + 1]) result.games = Number(args[++i]);
		else if (a === '--sweeps' && args[i + 1]) result.sweeps = Number(args[++i]);
		else if (a === '--step' && args[i + 1]) result.step = Number(args[++i]);
	}
	return result;
}

// ── Checkpoint I/O ─────────────────────────────────────────────────────────────

type Checkpoint = {
	meta: {
		sweepsCompleted: number;
		gamesPerEval: number;
		savedAt: string;
		bestGlobalScore: number;
	};
	params: Record<Level, DifficultyParams>;
};

function loadCheckpoint(): Record<Level, DifficultyParams> | null {
	if (!existsSync(CHECKPOINT_PATH)) return null;
	try {
		const raw = JSON.parse(readFileSync(CHECKPOINT_PATH, 'utf8')) as Checkpoint;
		console.log(
			`  Resumed from checkpoint (${raw.meta.sweepsCompleted} sweeps, saved ${raw.meta.savedAt})`
		);
		return raw.params as Record<Level, DifficultyParams>;
	} catch {
		console.warn('  Could not parse checkpoint — starting fresh');
		return null;
	}
}

function saveCheckpoint(
	params: Record<Level, DifficultyParams>,
	sweepsCompleted: number,
	bestGlobalScore: number
): void {
	mkdirSync('scripts/params', { recursive: true });
	const cp: Checkpoint = {
		meta: {
			sweepsCompleted,
			gamesPerEval: GAMES_PER_EVAL,
			savedAt: new Date().toISOString(),
			bestGlobalScore
		},
		params
	};
	writeFileSync(CHECKPOINT_PATH, JSON.stringify(cp, null, 2));
}

// ── Game simulation ────────────────────────────────────────────────────────────

function makeParamPlayer(index: number, params: DifficultyParams): Player & { _params: DifficultyParams } {
	return {
		id: createPlayerId(`p${index}`),
		kind: 'computer' as const,
		name: `p${index}`,
		difficulty: 'pikin' as const, // unused — params drive selection
		hand: [],
		_params: params
	};
}

function simulateOne(
	levelA: Level,
	levelB: Level,
	paramsA: DifficultyParams,
	paramsB: DifficultyParams,
	aFirst: boolean
): 'A' | 'B' | null {
	const [first, second, firstParams, secondParams] = aFirst
		? ([levelA, levelB, paramsA, paramsB] as const)
		: ([levelB, levelA, paramsB, paramsA] as const);

	// We only need ids and params; kind/difficulty are ignored in param-driven play
	const p0Id = createPlayerId('p0');
	const p1Id = createPlayerId('p1');

	const players: Player[] = [
		{ id: p0Id, kind: 'computer', name: first, difficulty: 'pikin', hand: [] },
		{ id: p1Id, kind: 'computer', name: second, difficulty: 'pikin', hand: [] }
	];

	let state = createGame(players, MODE);
	let turns = 0;

	while (state.phase === 'playing' && turns++ < MAX_TURNS) {
		const idx = state.currentPlayerIndex;
		const params = idx === 0 ? firstParams : secondParams;

		try {
			const action = selectMoveWithParams(state, idx, params);
			state = action === 'draw' ? drawCard(state, idx) : playCard(state, idx, action);
		} catch {
			state = { ...state, currentPlayerIndex: (idx + 1) % 2 };
		}
	}

	if (!state.winner) return null;
	const winnerId = state.winner.id;
	const aWon = aFirst ? winnerId === p0Id : winnerId === p1Id;
	return aWon ? 'A' : 'B';
}

/** Run n games and return win rate for A (timeouts excluded). */
function winRate(
	levelA: Level,
	levelB: Level,
	paramsA: DifficultyParams,
	paramsB: DifficultyParams,
	n: number
): number {
	let winsA = 0;
	let total = 0;
	for (let i = 0; i < n; i++) {
		const result = simulateOne(levelA, levelB, paramsA, paramsB, i % 2 === 0);
		if (result === 'A') { winsA++; total++; }
		else if (result === 'B') total++;
	}
	return total === 0 ? 0.5 : winsA / total;
}

// ── Objective function ─────────────────────────────────────────────────────────

/**
 * Objective for a single level: mean win rate vs all lower levels
 * minus mean win rate vs all higher levels.
 * Want this to be positive and large (ideally ~0.1–0.3).
 */
function objectiveForLevel(
	level: Level,
	params: Record<Level, DifficultyParams>,
	gamesPerMatchup: number
): number {
	const pos = LADDER.indexOf(level);
	const lower = LADDER.slice(0, pos);
	const higher = LADDER.slice(pos + 1);

	const vsLowerRates = lower.map((opp) =>
		winRate(level, opp, params[level], params[opp], gamesPerMatchup)
	);
	const vsHigherRates = higher.map((opp) =>
		winRate(level, opp, params[level], params[opp], gamesPerMatchup)
	);

	const avgVsLower = vsLowerRates.length > 0
		? vsLowerRates.reduce((s, r) => s + r, 0) / vsLowerRates.length
		: 0.5;
	const avgVsHigher = vsHigherRates.length > 0
		? vsHigherRates.reduce((s, r) => s + r, 0) / vsHigherRates.length
		: 0.5;

	return avgVsLower - avgVsHigher;
}

/**
 * Quick adjacent-only objective: only vs the level immediately below and above.
 * Used during param sweeps for speed.
 * Subtracts TARGET_MARGIN from the vs-lower term so the optimizer keeps pushing
 * even after direction is correct — it won't stop until margins are wide enough.
 */
function quickObjective(
	level: Level,
	params: Record<Level, DifficultyParams>,
	gamesPerMatchup: number
): number {
	const pos = LADDER.indexOf(level);
	let score = 0;
	let terms = 0;

	if (pos > 0) {
		const below = LADDER[pos - 1]!;
		// Reward exceeding TARGET_MARGIN, penalise falling short of it
		score += winRate(level, below, params[level], params[below], gamesPerMatchup) - TARGET_MARGIN;
		terms++;
	}
	if (pos < LADDER.length - 1) {
		const above = LADDER[pos + 1]!;
		score -= winRate(level, above, params[level], params[above], gamesPerMatchup);
		terms++;
	}

	return terms > 0 ? score / terms * 2 : 0;
}

// ── Multiplayer simulation ─────────────────────────────────────────────────────

/** Run one n-player game with explicit params; return winner index or null on timeout. */
function simulateMultiN(levels: Level[], params: Record<Level, DifficultyParams>): number | null {
	const playerIds = levels.map((_, i) => createPlayerId(`p${i}`));
	const players: Player[] = levels.map((lvl, i) => ({
		id: playerIds[i]!,
		kind: 'computer' as const,
		name: lvl,
		difficulty: 'pikin' as const,
		hand: []
	}));
	let state = createGame(players, MODE);
	let turns = 0;
	const n = players.length;
	while (state.phase === 'playing' && turns++ < MAX_TURNS) {
		const idx = state.currentPlayerIndex;
		const levelParams = params[levels[idx]!]!;
		try {
			const action = selectMoveWithParams(state, idx, levelParams);
			state = action === 'draw' ? drawCard(state, idx) : playCard(state, idx, action);
		} catch {
			state = { ...state, currentPlayerIndex: (idx + 1) % n };
		}
	}
	if (!state.winner) return null;
	return playerIds.indexOf(state.winner.id);
}

/**
 * Pick 2 filler levels spread across the ladder, avoiding hi and lo.
 * Gives mixed-level context for 4-player games.
 */
function pickFillers(hi: Level, lo: Level): [Level, Level] {
	const pool = LADDER.filter((l) => l !== hi && l !== lo);
	// Take one from the lower half and one from the upper half of the filtered pool
	const mid = Math.floor(pool.length / 2);
	return [pool[mid - 1] ?? pool[0]!, pool[mid + 1] ?? pool[pool.length - 1]!];
}

/**
 * In n-game 4-player sessions, return P(hi wins) / (P(hi wins) + P(lo wins)).
 * Fillers create competitive context without biasing the hi/lo comparison.
 */
function multiplayerPairRate(
	hi: Level,
	lo: Level,
	params: Record<Level, DifficultyParams>,
	n: number
): number {
	const [f1, f2] = pickFillers(hi, lo);
	const baseLevels: Level[] = [hi, lo, f1, f2];
	let hiWins = 0;
	let loWins = 0;
	for (let i = 0; i < n; i++) {
		// Shuffle seating each game to remove first-player bias
		const seated = [...baseLevels].sort(() => Math.random() - 0.5) as Level[];
		const winIdx = simulateMultiN(seated, params);
		if (winIdx === null) continue;
		const winner = seated[winIdx]!;
		if (winner === hi) hiWins++;
		else if (winner === lo) loWins++;
	}
	const total = hiWins + loWins;
	return total === 0 ? 0.5 : hiWins / total;
}

/**
 * Global multiplayer ordering score: for every (hi, lo) pair, measure how
 * dominant hi is in 4-player games. Returns a 0–1 continuous score where
 * 1.0 means every pair reaches TARGET_MARGIN in multiplayer too.
 */
function multiplayerGlobalScore(params: Record<Level, DifficultyParams>, gamesPerPair: number): number {
	let totalScore = 0;
	let pairs = 0;
	for (let i = 0; i < LADDER.length; i++) {
		for (let j = i + 1; j < LADDER.length; j++) {
			const lo = LADDER[i]!;
			const hi = LADDER[j]!;
			const rate = multiplayerPairRate(hi, lo, params, gamesPerPair);
			// Continuous: 0 at rate=0.5, 1.0 at rate=TARGET_MARGIN
			totalScore += Math.min(1, Math.max(0, (rate - 0.5) / (TARGET_MARGIN - 0.5)));
			pairs++;
		}
	}
	return pairs > 0 ? totalScore / pairs : 0;
}

// ── Global ranking score ───────────────────────────────────────────────────────

/**
 * Full tournament: run every pair and count how many exceed TARGET_MARGIN.
 * Blends 2-player (65%) and 4-player (35%) scores.
 * Returns the combined score (0–1) and the 2-player matrix for display.
 */
function globalRankingScore(
	params: Record<Level, DifficultyParams>,
	gamesPerMatchup: number
): { score: number; matrix: Record<Level, Record<Level, number>> } {
	const matrix = {} as Record<Level, Record<Level, number>>;
	for (const l of LADDER) matrix[l] = {} as Record<Level, number>;

	let correct = 0;
	let total = 0;

	for (let i = 0; i < LADDER.length; i++) {
		for (let j = i + 1; j < LADDER.length; j++) {
			const lo = LADDER[i]!;
			const hi = LADDER[j]!;
			const rate = winRate(hi, lo, params[hi], params[lo], gamesPerMatchup);
			matrix[hi][lo] = rate;
			matrix[lo][hi] = 1 - rate;
			// Only counts as "correct" when margin exceeds the target, not just direction
			if (rate > TARGET_MARGIN) correct++;
			total++;
		}
	}

	const score2p = total > 0 ? correct / total : 0;
	const multiGames = Math.max(50, Math.floor(gamesPerMatchup * MULTI_GAMES_RATIO));
	const scoreMulti = multiplayerGlobalScore(params, multiGames);
	const combined = (1 - MULTI_WEIGHT) * score2p + MULTI_WEIGHT * scoreMulti;

	return { score: combined, matrix };
}

// ── Coordinate descent ─────────────────────────────────────────────────────────

function printMatrix(
	matrix: Record<Level, Record<Level, number>>,
	params: Record<Level, DifficultyParams>
): void {
	const COL = 11;
	const pad = (s: string) => s.padStart(COL);
	const pct = (v: number | undefined) =>
		v === undefined ? '   —   ' : `${(v * 100).toFixed(1)}%`;

	process.stdout.write(' '.repeat(14));
	for (const col of LADDER) process.stdout.write(pad(col));
	console.log();
	console.log('─'.repeat(14 + LADDER.length * COL));

	for (const row of LADDER) {
		process.stdout.write(row.padEnd(14));
		for (const col of LADDER) {
			if (row === col) process.stdout.write(pad('·'));
			else process.stdout.write(pad(pct(matrix[row]?.[col])));
		}
		console.log();
	}
	console.log('─'.repeat(14 + LADDER.length * COL));
}

function printParams(params: Record<Level, DifficultyParams>): void {
	const keys: (keyof DifficultyParams)[] = [...PARAM_KEYS];
	const KCOL = 16;
	const VCOL = 8;

	process.stdout.write(' '.repeat(KCOL));
	for (const lvl of TUNABLE) process.stdout.write(lvl.padStart(VCOL + 2));
	console.log();
	console.log('─'.repeat(KCOL + (VCOL + 2) * TUNABLE.length));

	for (const key of keys) {
		process.stdout.write(key.padEnd(KCOL));
		for (const lvl of TUNABLE) {
			const v = params[lvl][key];
			process.stdout.write(v.toFixed(3).padStart(VCOL + 2));
		}
		console.log();
	}
}

async function main(): Promise<void> {
	console.log('\n╔══════════════════════════════════════════════════════════╗');
	console.log('║  WHOTS AI TUNER                                          ║');
	console.log('╚══════════════════════════════════════════════════════════╝\n');
	console.log(`  Games per eval  : ${GAMES_PER_EVAL}`);
	console.log(`  Max sweeps      : ${ARGS.continuous ? '∞ (continuous mode)' : MAX_SWEEPS}`);
	console.log(`  Initial step    : ${INITIAL_STEP}`);
	console.log(`  Mode            : ${MODE}`);
	console.log(`  Continuous      : ${ARGS.continuous ? 'yes — restarts on convergence until Ctrl+C' : 'no'}`);
	console.log(`  Checkpoint      : ${CHECKPOINT_PATH}`);
	console.log();

	// Load or initialize params
	let params: Record<Level, DifficultyParams>;
	if (ARGS.resume) {
		params = loadCheckpoint() ?? ({ ...DEFAULT_PARAMS } as Record<Level, DifficultyParams>);
	} else {
		params = { ...DEFAULT_PARAMS } as Record<Level, DifficultyParams>;
	}

	// ── Verify-only mode ────────────────────────────────────────────────────
	if (ARGS.verify) {
		console.log('── Verification run ────────────────────────────────────────\n');
		const verifyGames = Math.max(GAMES_PER_EVAL, 1000);
		const { score, matrix } = globalRankingScore(params, verifyGames);
		printMatrix(matrix, params);
		console.log(`\n  Ordering score: ${(score * 100).toFixed(1)}% pairs correct`);
		console.log();
		return;
	}

	// ── Baseline evaluation ─────────────────────────────────────────────────
	console.log('── Baseline evaluation ─────────────────────────────────────\n');
	const { score: baseScore, matrix: baseMatrix } = globalRankingScore(params, GAMES_PER_EVAL);
	printMatrix(baseMatrix, params);
	console.log(`\n  Baseline ordering score: ${(baseScore * 100).toFixed(1)}%\n`);

	let bestParams = structuredClone(params);
	let bestGlobalScore = baseScore;     // local acceptance threshold for this restart
	let allTimeBestParams = structuredClone(params);
	let allTimeBestScore = baseScore;    // all-time best — only this gets saved to checkpoint
	let step = INITIAL_STEP;
	let totalSweeps = 0;
	let restartCount = 0;

	// Graceful shutdown on Ctrl+C
	let interrupted = false;
	process.on('SIGINT', () => {
		interrupted = true;
		console.log('\n\n  Ctrl+C received — saving and exiting...');
	});

	// ── Main optimization loop ───────────────────────────────────────────────
	for (let sweep = 1; sweep <= MAX_SWEEPS && !interrupted; sweep++) {
		totalSweeps++;
		const sweepLabel = ARGS.continuous
			? `Sweep ${sweep}/${MAX_SWEEPS}  restart #${restartCount}  total ${totalSweeps}`
			: `Sweep ${sweep}/${MAX_SWEEPS}`;
		console.log(`\n── ${sweepLabel}  (step=${step.toFixed(4)}) ─────────────────────────────\n`);

		let improvedThisSweep = false;

		for (const level of TUNABLE) {
			if (interrupted) break;

			// Baseline quick-objective for this level
			let bestLevelScore = quickObjective(level, bestParams, GAMES_PER_EVAL);
			let bestLevelParams = { ...bestParams[level] };

			for (const key of PARAM_KEYS) {
				if (interrupted) break;

				const current = bestLevelParams[key];
				const [lo, hi] = PARAM_BOUNDS[key];

				// Try +step and -step
				for (const delta of [step, -step]) {
					const candidate = clampParams({ ...bestLevelParams, [key]: current + delta });
					const testParams = { ...bestParams, [level]: candidate };
					const score = quickObjective(level, testParams, GAMES_PER_EVAL);

					if (score > bestLevelScore + 0.001) {
						const delta = score - bestLevelScore;
						bestLevelScore = score;
						bestLevelParams = candidate;
						process.stdout.write(
							`  ${level.padEnd(12)} ${key.padEnd(18)} ${current.toFixed(3)} → ${candidate[key].toFixed(3)}  (Δobj ${(delta * 100).toFixed(1)}%)\n`
						);
					}
				}
			}

			// Accept improvements for this level, then re-check global ranking
			if (JSON.stringify(bestLevelParams) !== JSON.stringify(bestParams[level])) {
				bestParams = { ...bestParams, [level]: bestLevelParams };

				// Full global re-evaluation after updating this level
				const { score: newGlobal } = globalRankingScore(bestParams, GAMES_PER_EVAL);
				if (newGlobal >= bestGlobalScore) {
					bestGlobalScore = newGlobal;
					improvedThisSweep = true;
					if (newGlobal > allTimeBestScore) {
						allTimeBestScore = newGlobal;
						allTimeBestParams = structuredClone(bestParams);
						saveCheckpoint(allTimeBestParams, totalSweeps, allTimeBestScore);
						console.log(
							`  ✓ ${level}: new all-time best ${(newGlobal * 100).toFixed(1)}%  [saved]\n`
						);
					} else {
						console.log(
							`  ✓ ${level}: local improvement ${(newGlobal * 100).toFixed(1)}%\n`
						);
					}
				} else {
					// Revert — this change hurt the global ordering
					bestParams = { ...bestParams, [level]: params[level] };
					console.log(`  ✗ ${level}: reverted (global dropped to ${(newGlobal * 100).toFixed(1)}%)\n`);
				}
			}
		}

		// ── End of sweep ─────────────────────────────────────────────────────
		if (!improvedThisSweep) {
			if (step > MIN_STEP) {
				step *= STEP_DECAY;
				console.log(`  No improvement — step → ${step.toFixed(4)}`);
			} else if (ARGS.continuous) {
				// Converged — perturb the all-time best params and restart
				restartCount++;
				console.log(`\n  Converged — restart #${restartCount} (perturbing all-time best)\n`);
				const perturbStrength = PERTURB_STRENGTH;
				const perturbed = structuredClone(allTimeBestParams);
				for (const level of TUNABLE) {
					for (const key of PARAM_KEYS) {
						const jitter = (Math.random() * 2 - 1) * perturbStrength;
						perturbed[level][key] = allTimeBestParams[level][key] + jitter;
					}
					perturbed[level] = clampParams(perturbed[level]);
				}
				params = perturbed;
				bestParams = structuredClone(perturbed);
				// Use the perturbed baseline as the local acceptance threshold so this
				// restart can make progress even if it starts below the all-time best
				const { score: perturbedBaseline } = globalRankingScore(perturbed, GAMES_PER_EVAL);
				bestGlobalScore = perturbedBaseline;
				step = INITIAL_STEP;
				sweep = 0; // will be incremented to 1 at loop top
			} else {
				console.log('\n  Converged (step at minimum, no improvement).');
				break;
			}
		}
	}

	// ── Final summary ────────────────────────────────────────────────────────
	console.log('\n\n══ Final results ═══════════════════════════════════════════\n');

	const finalGames = Math.max(GAMES_PER_EVAL * 4, 2000);
	console.log(`Running final verification with ${finalGames} games per matchup...\n`);
	const { score: finalScore, matrix: finalMatrix } = globalRankingScore(allTimeBestParams, finalGames);
	printMatrix(finalMatrix, allTimeBestParams);

	console.log('\n── Tuned parameters ─────────────────────────────────────────\n');
	printParams(allTimeBestParams);

	console.log(`\n  Final ordering score : ${(finalScore * 100).toFixed(1)}% pairs correct`);
	console.log(`  All-time best (tuner): ${(allTimeBestScore * 100).toFixed(1)}%`);
	if (ARGS.continuous) console.log(`  Total restarts       : ${restartCount}`);
	console.log(`  Total sweeps         : ${totalSweeps}`);
	console.log(`\n  Results saved to     : ${CHECKPOINT_PATH}\n`);

	saveCheckpoint(allTimeBestParams, totalSweeps, finalScore);
}

main().catch(console.error);
