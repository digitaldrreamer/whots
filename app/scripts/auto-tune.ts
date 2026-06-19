/**
 * ╔══════════════════════════════════════════════════════════════════════════╗
 * ║  WHOTS AI AUTONOMOUS TUNER                                              ║
 * ║                                                                         ║
 * ║  Runs coordinate-descent + iterated restarts across 3 phases of        ║
 * ║  increasing simulation fidelity. Stops only when the high standard     ║
 * ║  is met:                                                                ║
 * ║    • All 21 difficulty pairs in correct order at 10,000 games each     ║
 * ║    • Every adjacent pair separated by ≥ 2 percentage points            ║
 * ║                                                                         ║
 * ║  Usage:                                                                 ║
 * ║    npx tsx scripts/auto-tune.ts            # fresh run                 ║
 * ║    npx tsx scripts/auto-tune.ts --resume   # continue from checkpoint  ║
 * ║                                                                         ║
 * ║  Ctrl+C at any time — best params are saved after every improvement.   ║
 * ╚══════════════════════════════════════════════════════════════════════════╝
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { createGame, createPlayerId, drawCard, playCard } from '../src/lib/game/state.js';
import type { GameMode, Player } from '../src/lib/game/types.js';
import {
	DEFAULT_PARAMS,
	PARAM_KEYS,
	PARAM_BOUNDS,
	clampParams,
	selectMoveWithParams
} from '../src/lib/game/computer/params.js';
import type { DifficultyParams } from '../src/lib/game/computer/params.js';

// ── Constants ──────────────────────────────────────────────────────────────────

const CHECKPOINT_PATH = 'scripts/params/best.json';
const MODE: GameMode = 'stack';
const MAX_TURNS = 600;

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
const TUNABLE: Level[] = ['smallz', 'isabiSmall', 'chief', 'egbon', 'jagaban'];

// Coordinate-descent hyperparams (same as tune.ts)
const INITIAL_STEP = 0.15;
const MIN_STEP = 0.005;
const STEP_DECAY = 0.6;
const PERTURB_SIGMA = 0.25; // ±σ applied to each param when restarting

// ── Phase ladder ───────────────────────────────────────────────────────────────
//
// Each phase runs coordinate-descent + random restarts until `staleAfter`
// consecutive restarts produce no improvement to the all-time best score.
// Higher game counts give cleaner signal at the cost of slower iterations.

type Phase = { games: number; staleAfter: number; label: string };

const PHASES: Phase[] = [
	{ games: 500, staleAfter: 20, label: 'Phase 1 — Explore  (500 games/eval)' },
	{ games: 2000, staleAfter: 15, label: 'Phase 2 — Refine   (2000 games/eval)' },
	{ games: 5000, staleAfter: 10, label: 'Phase 3 — Polish   (5000 games/eval)' }
];

// ── High-standard definition ───────────────────────────────────────────────────

const VERIFY_GAMES = 10_000; // games used for the final quality check
const REQUIRED_ORDERING = 1.0; // all 21 pairs correct (fraction)
const MIN_ADJ_MARGIN = 0.02; // each adjacent pair: higher must win by ≥ 52%

// ── Checkpoint ─────────────────────────────────────────────────────────────────

type Checkpoint = {
	meta: {
		savedAt: string;
		phase: string;
		totalRestarts: number;
		score: number;
	};
	params: Record<Level, DifficultyParams>;
};

function saveCheckpoint(
	params: Record<Level, DifficultyParams>,
	phase: string,
	totalRestarts: number,
	score: number
): void {
	mkdirSync('scripts/params', { recursive: true });
	const cp: Checkpoint = {
		meta: { savedAt: new Date().toISOString(), phase, totalRestarts, score },
		params
	};
	writeFileSync(CHECKPOINT_PATH, JSON.stringify(cp, null, 2));
}

function loadCheckpoint(): Record<Level, DifficultyParams> | null {
	if (!existsSync(CHECKPOINT_PATH)) return null;
	try {
		const raw = JSON.parse(readFileSync(CHECKPOINT_PATH, 'utf8')) as Checkpoint;
		console.log(
			`  Loaded checkpoint — phase: ${raw.meta.phase}, score: ${(raw.meta.score * 100).toFixed(1)}%`
		);
		console.log(`  Saved: ${raw.meta.savedAt}\n`);
		return raw.params as Record<Level, DifficultyParams>;
	} catch {
		console.warn('  Checkpoint unreadable — starting fresh\n');
		return null;
	}
}

// ── Simulation ─────────────────────────────────────────────────────────────────

function simulateGame(
	paramsA: DifficultyParams,
	paramsB: DifficultyParams,
	aFirst: boolean
): 'A' | 'B' | null {
	const idA = createPlayerId('pA');
	const idB = createPlayerId('pB');

	// Player order depends on who goes first
	const players: Player[] = aFirst
		? [
				{ id: idA, kind: 'computer', name: 'A', difficulty: 'pikin', hand: [] },
				{ id: idB, kind: 'computer', name: 'B', difficulty: 'pikin', hand: [] }
			]
		: [
				{ id: idB, kind: 'computer', name: 'B', difficulty: 'pikin', hand: [] },
				{ id: idA, kind: 'computer', name: 'A', difficulty: 'pikin', hand: [] }
			];

	let state = createGame(players, MODE);
	let turns = 0;

	while (state.phase === 'playing' && turns++ < MAX_TURNS) {
		const idx = state.currentPlayerIndex;
		// Map player index → params regardless of seat order
		const isA = state.players[idx]?.id === idA;
		const params = isA ? paramsA : paramsB;
		try {
			const action = selectMoveWithParams(state, idx, params);
			state = action === 'draw' ? drawCard(state, idx) : playCard(state, idx, action);
		} catch {
			state = { ...state, currentPlayerIndex: (idx + 1) % 2 };
		}
	}

	if (!state.winner) return null;
	return state.winner.id === idA ? 'A' : 'B';
}

/** Run n games alternating first-player; return A's win rate. */
function winRate(paramsA: DifficultyParams, paramsB: DifficultyParams, n: number): number {
	let wA = 0;
	let total = 0;
	for (let i = 0; i < n; i++) {
		const r = simulateGame(paramsA, paramsB, i % 2 === 0);
		if (r === 'A') {
			wA++;
			total++;
		} else if (r === 'B') {
			total++;
		}
	}
	return total === 0 ? 0.5 : wA / total;
}

/** Adjacent-only quick objective (fast; used during coordinate descent sweeps). */
function quickObjective(
	level: Level,
	params: Record<Level, DifficultyParams>,
	n: number
): number {
	const pos = LADDER.indexOf(level);
	let score = 0;
	let terms = 0;
	if (pos > 0) {
		const below = LADDER[pos - 1]!;
		score += winRate(params[level], params[below], n);
		terms++;
	}
	if (pos < LADDER.length - 1) {
		const above = LADDER[pos + 1]!;
		score -= winRate(params[level], params[above], n);
		terms++;
	}
	return terms > 0 ? (score / terms) * 2 : 0;
}

/** Full tournament over all C(7,2)=21 pairs. Returns fraction in correct order. */
function globalScore(
	params: Record<Level, DifficultyParams>,
	n: number
): { score: number; matrix: Record<Level, Record<Level, number>> } {
	const matrix = Object.fromEntries(LADDER.map((l) => [l, {}])) as Record<
		Level,
		Record<Level, number>
	>;
	let correct = 0;
	let total = 0;

	for (let i = 0; i < LADDER.length; i++) {
		for (let j = i + 1; j < LADDER.length; j++) {
			const lo = LADDER[i]!;
			const hi = LADDER[j]!;
			const rate = winRate(params[hi], params[lo], n);
			matrix[hi][lo] = rate;
			matrix[lo][hi] = 1 - rate;
			if (rate > 0.5) correct++;
			total++;
		}
	}

	return { score: total > 0 ? correct / total : 0, matrix };
}

// ── Coordinate descent ─────────────────────────────────────────────────────────

/**
 * Run one full coordinate-descent convergence from startParams.
 * Returns the best params found and their global score at the given game count.
 * Uses localBaseline as the acceptance threshold so each restart can make
 * progress even from a perturbed starting point below the all-time best.
 */
function coordinateDescent(
	startParams: Record<Level, DifficultyParams>,
	localBaseline: number,
	games: number
): { params: Record<Level, DifficultyParams>; score: number } {
	let params = { ...startParams };
	let bestScore = localBaseline;
	let step = INITIAL_STEP;

	while (step >= MIN_STEP) {
		let improvedThisSweep = false;

		for (const level of TUNABLE) {
			// Snapshot state before attempting this level
			const snapshot = { ...params };

			let bestLevelParams = { ...params[level] };
			let bestLevelScore = quickObjective(level, params, games);

			for (const key of PARAM_KEYS) {
				const cur = bestLevelParams[key];
				for (const delta of [step, -step]) {
					const candidate = clampParams({ ...bestLevelParams, [key]: cur + delta });
					const s = quickObjective(level, { ...params, [level]: candidate }, games);
					if (s > bestLevelScore + 0.001) {
						bestLevelScore = s;
						bestLevelParams = candidate;
					}
				}
			}

			// If the level improved, run a global check before accepting
			if (JSON.stringify(bestLevelParams) !== JSON.stringify(params[level])) {
				const testParams = { ...params, [level]: bestLevelParams };
				const { score: g } = globalScore(testParams, games);
				if (g >= bestScore) {
					params = testParams;
					bestScore = g;
					improvedThisSweep = true;
				} else {
					params = snapshot; // revert all changes to this level
				}
			}
		}

		if (!improvedThisSweep) step *= STEP_DECAY;
	}

	return { params, score: bestScore };
}

// ── Perturbation ───────────────────────────────────────────────────────────────

function perturb(base: Record<Level, DifficultyParams>): Record<Level, DifficultyParams> {
	const result = { ...base };
	for (const level of TUNABLE) {
		const p = { ...base[level] };
		for (const key of PARAM_KEYS) {
			const [lo, hi] = PARAM_BOUNDS[key];
			p[key] = Math.max(lo, Math.min(hi, base[level][key] + (Math.random() * 2 - 1) * PERTURB_SIGMA));
		}
		result[level] = p;
	}
	return result;
}

// ── Standard check ─────────────────────────────────────────────────────────────

type StandardResult = {
	met: boolean;
	orderingScore: number;
	issues: string[];
	matrix: Record<Level, Record<Level, number>>;
};

function checkStandard(
	params: Record<Level, DifficultyParams>,
	games: number
): StandardResult {
	const { score, matrix } = globalScore(params, games);
	const issues: string[] = [];

	// Check all pairs
	for (let i = 0; i < LADDER.length; i++) {
		for (let j = i + 1; j < LADDER.length; j++) {
			const lo = LADDER[i]!;
			const hi = LADDER[j]!;
			const rate = matrix[hi]?.[lo] ?? 0;
			if (rate <= 0.5) issues.push(`${lo} beats ${hi} (${(rate * 100).toFixed(1)}%)`);
		}
	}

	// Check adjacent margins
	for (let i = 0; i < LADDER.length - 1; i++) {
		const lo = LADDER[i]!;
		const hi = LADDER[i + 1]!;
		const rate = matrix[hi]?.[lo] ?? 0;
		const margin = rate - 0.5;
		if (margin < MIN_ADJ_MARGIN)
			issues.push(
				`${hi} vs ${lo}: margin only ${(margin * 100).toFixed(1)}pp (need ≥${MIN_ADJ_MARGIN * 100}pp)`
			);
	}

	return { met: issues.length === 0, orderingScore: score, issues, matrix };
}

// ── Display ────────────────────────────────────────────────────────────────────

function printMatrix(matrix: Record<Level, Record<Level, number>>): void {
	const COL = 11;
	const pad = (s: string) => s.padStart(COL);
	const pct = (v: number | undefined) =>
		v === undefined ? '    ·   ' : `${(v * 100).toFixed(1).padStart(5)}%`;

	process.stdout.write(' '.repeat(14));
	for (const col of LADDER) process.stdout.write(pad(col));
	console.log();
	console.log('─'.repeat(14 + LADDER.length * COL));
	for (const row of LADDER) {
		process.stdout.write(row.padEnd(14));
		for (const col of LADDER) {
			process.stdout.write(row === col ? pad('·') : pad(pct(matrix[row]?.[col])));
		}
		console.log();
	}
	console.log('─'.repeat(14 + LADDER.length * COL));
}

function printParams(params: Record<Level, DifficultyParams>): void {
	const KCOL = 16;
	const VCOL = 9;
	process.stdout.write(' '.repeat(KCOL));
	for (const lvl of TUNABLE) process.stdout.write(lvl.padStart(VCOL + 2));
	console.log();
	console.log('─'.repeat(KCOL + (VCOL + 2) * TUNABLE.length));
	for (const key of PARAM_KEYS) {
		process.stdout.write(key.padEnd(KCOL));
		for (const lvl of TUNABLE)
			process.stdout.write(params[lvl][key].toFixed(3).padStart(VCOL + 2));
		console.log();
	}
}

function printBanner(line: string): void {
	const width = 70;
	console.log('\n' + '═'.repeat(width));
	console.log('  ' + line);
	console.log('═'.repeat(width) + '\n');
}

// ── Main ───────────────────────────────────────────────────────────────────────

async function main(): Promise<void> {
	const resume = process.argv.includes('--resume');
	let interrupted = false;
	process.on('SIGINT', () => {
		interrupted = true;
		console.log('\n\n  Ctrl+C — will stop after this restart completes...');
	});

	console.log('\n╔══════════════════════════════════════════════════════════════════════╗');
	console.log('║  WHOTS AI — AUTONOMOUS TUNER                                        ║');
	console.log('╠══════════════════════════════════════════════════════════════════════╣');
	console.log('║  Target: 100% ordering + ≥2pp adjacent gaps at 10,000 games/pair   ║');
	console.log('╚══════════════════════════════════════════════════════════════════════╝\n');

	// Initialise params from checkpoint or defaults
	let allTimeBestParams: Record<Level, DifficultyParams>;
	if (resume) {
		console.log('  Loading checkpoint...');
		allTimeBestParams =
			loadCheckpoint() ?? (DEFAULT_PARAMS as Record<Level, DifficultyParams>);
	} else {
		allTimeBestParams = DEFAULT_PARAMS as Record<Level, DifficultyParams>;
	}

	// Evaluate starting params at phase-1 fidelity
	process.stdout.write('  Evaluating starting params (500 games)... ');
	const { score: initScore } = globalScore(allTimeBestParams, 500);
	let allTimeBestScore = initScore;
	console.log(`${(initScore * 100).toFixed(1)}% pairs correct\n`);

	let totalRestarts = 0;
	let cycleCount = 0;
	const runStart = Date.now();

	// Outer cycle: runs until standard is met or Ctrl+C
	while (!interrupted) {
		cycleCount++;

		// First cycle runs all 3 phases; subsequent cycles skip phase 1
		// (coarse exploration already done — go straight to refinement)
		const phases = cycleCount === 1 ? PHASES : PHASES.slice(1);

		for (const phase of phases) {
			if (interrupted) break;

			printBanner(`${phase.label}  [cycle ${cycleCount}]`);
			console.log(`  Stale-stop: ${phase.staleAfter} consecutive non-improving restarts\n`);

			let stale = 0;
			let phaseRestarts = 0;

			while (stale < phase.staleAfter && !interrupted) {
				const isVeryFirst = cycleCount === 1 && phase === PHASES[0] && phaseRestarts === 0;

				// First attempt of the very first phase uses params as-is;
				// all subsequent restarts add random perturbation
				const startParams = isVeryFirst ? allTimeBestParams : perturb(allTimeBestParams);

				// Evaluate the starting point as a local baseline so this restart
				// can make progress even if starting below the all-time best
				const { score: localBaseline } = globalScore(startParams, phase.games);

				const elapsed = ((Date.now() - runStart) / 60_000).toFixed(1);
				process.stdout.write(
					`  [restart ${String(totalRestarts).padStart(4)}]` +
						`  stale ${stale}/${phase.staleAfter}` +
						`  local ${(localBaseline * 100).toFixed(1)}%` +
						`  best ${(allTimeBestScore * 100).toFixed(1)}%` +
						`  ${elapsed}min  ...`
				);

				// Run coordinate descent from this starting point
				const { params: improved, score: newScore } = coordinateDescent(
					startParams,
					localBaseline,
					phase.games
				);

				if (newScore > allTimeBestScore) {
					allTimeBestScore = newScore;
					allTimeBestParams = improved;
					stale = 0;
					saveCheckpoint(allTimeBestParams, phase.label, totalRestarts, allTimeBestScore);
					console.log(`  ✓ NEW BEST ${(newScore * 100).toFixed(1)}%  [saved]`);
				} else {
					stale++;
					console.log(`  · no improvement`);
				}

				totalRestarts++;
				phaseRestarts++;
			}

			if (stale >= phase.staleAfter) {
				console.log(`\n  Phase complete — ${phase.staleAfter} stale restarts reached.\n`);
			}
		}

		if (interrupted) break;

		// ── High-standard verification ─────────────────────────────────────────
		printBanner(`Cycle ${cycleCount} — Verification at ${VERIFY_GAMES.toLocaleString()} games/pair`);
		console.log('  Running final tournament (this takes several minutes)...\n');

		const { met, orderingScore, issues, matrix } = checkStandard(
			allTimeBestParams,
			VERIFY_GAMES
		);

		printMatrix(matrix);
		console.log('\n── Tuned parameters ──────────────────────────────────────────────\n');
		printParams(allTimeBestParams);

		console.log(`\n  Ordering score : ${(orderingScore * 100).toFixed(1)}% (need 100%)`);
		console.log(`  Adjacent gaps  : need ≥${MIN_ADJ_MARGIN * 100}pp on all 6 adjacent pairs`);
		console.log(`  Total restarts : ${totalRestarts}`);
		console.log(`  Total time     : ${((Date.now() - runStart) / 60_000).toFixed(1)} min`);

		if (met) {
			console.log('\n  ╔═══════════════════════════════════════╗');
			console.log('  ║   ✓  HIGH STANDARD MET — DONE!  ✓    ║');
			console.log('  ╚═══════════════════════════════════════╝\n');
			console.log('  Next steps:');
			console.log('  1. Params saved to scripts/params/best.json');
			console.log('  2. Copy the tuned values above into DEFAULT_PARAMS in:');
			console.log('       app/src/lib/game/computer/params.ts');
			console.log('  3. Confirm with: npx tsx scripts/simulate.ts\n');
			saveCheckpoint(allTimeBestParams, 'DONE — standard met', totalRestarts, orderingScore);
			break;
		}

		console.log(`\n  Standard not yet met. Failing checks:`);
		for (const issue of issues) console.log(`    • ${issue}`);
		console.log(`\n  Starting cycle ${cycleCount + 1} (phases 2–3, skipping explore)...\n`);
	}

	if (interrupted) {
		console.log('\n  Stopped. Best params are in scripts/params/best.json\n');
		console.log('  Resume with: npx tsx scripts/auto-tune.ts --resume\n');
	}
}

main().catch(console.error);
