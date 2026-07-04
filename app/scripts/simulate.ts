import { DIFFICULTIES } from '../src/lib/game/types.js';
import type { Difficulty, GameMode, GameState, Player } from '../src/lib/game/types.js';
import { createGame, createPlayerId, drawCard, playCard } from '../src/lib/game/state.js';
import type { PlayAction } from '../src/lib/game/state.js';
import { selectMove, selectMoveTeeNoble } from '../src/lib/game/computer/index.js';
import { shuffle } from '../src/lib/game/deck.js';

// --- Config ---

const GAMES_PER_MATCHUP = 2000;
const GAMES_PER_4P_COMBO = 200;
const MODE: GameMode = 'stack';
const MAX_TURNS = 600; // safety cap per game

// --- Types ---

type Level = Difficulty | 'tee-noble';
const ALL_LEVELS: Level[] = [...DIFFICULTIES, 'tee-noble'];

// --- Helpers ---

function makePlayer(index: number, level: Level): Player {
	if (level === 'tee-noble') {
		return { id: createPlayerId(`p${index}`), kind: 'tee-noble', name: 'Tee-Noble', hand: [] };
	}
	return {
		id: createPlayerId(`p${index}`),
		kind: 'computer',
		name: level,
		difficulty: level,
		hand: []
	};
}

function chooseAction(state: GameState, playerIndex: number, player: Player): PlayAction | 'draw' {
	if (player.kind === 'tee-noble') return selectMoveTeeNoble(state, playerIndex);
	if (player.kind === 'computer') return selectMove(state, playerIndex, player.difficulty);
	return 'draw';
}

function simulateGame(levelA: Level, levelB: Level): 0 | 1 | null {
	const playerA = makePlayer(0, levelA);
	const playerB = makePlayer(1, levelB);
	let state = createGame([playerA, playerB], MODE);
	let turns = 0;

	while (state.phase === 'playing' && turns++ < MAX_TURNS) {
		const idx = state.currentPlayerIndex;
		const player = state.players[idx];
		if (player === undefined) break;

		const action = chooseAction(state, idx, player);

		try {
			state = action === 'draw' ? drawCard(state, idx) : playCard(state, idx, action);
		} catch {
			// Shouldn't happen — if it does, force-advance so the sim doesn't hang
			state = { ...state, currentPlayerIndex: (idx + 1) % 2 };
		}
	}

	if (state.winner?.id === playerA.id) return 0;
	if (state.winner?.id === playerB.id) return 1;
	return null; // timeout / draw
}

// Returns the index within `levels` of the winner, or null on timeout
function simulateNPlayerGame(levels: Level[]): number | null {
	const players = levels.map((lvl, i) => makePlayer(i, lvl));
	let state = createGame(players, MODE);
	let turns = 0;
	const n = players.length;

	while (state.phase === 'playing' && turns++ < MAX_TURNS) {
		const idx = state.currentPlayerIndex;
		const player = state.players[idx];
		if (player === undefined) break;

		const action = chooseAction(state, idx, player);

		try {
			state = action === 'draw' ? drawCard(state, idx) : playCard(state, idx, action);
		} catch {
			state = { ...state, currentPlayerIndex: (idx + 1) % n };
		}
	}

	if (!state.winner) return null;
	return players.findIndex((p) => p.id === state.winner?.id);
}

function combinations<T>(items: T[], k: number): T[][] {
	if (k === 0) return [[]];
	if (items.length < k) return [];
	const [first, ...rest] = items;
	if (first === undefined) return [];
	return [
		...combinations(rest, k - 1).map((combo) => [first, ...combo]),
		...combinations(rest, k)
	];
}

function runMatchup(
	a: Level,
	b: Level,
	n: number
): { winsA: number; winsB: number; timeouts: number } {
	let winsA = 0,
		winsB = 0,
		timeouts = 0;
	for (let i = 0; i < n; i++) {
		// Alternate first player to eliminate seating bias
		const aFirst = i % 2 === 0;
		const r = simulateGame(aFirst ? a : b, aFirst ? b : a);
		if (r === 0) { if (aFirst) winsA++; else winsB++; }
		else if (r === 1) { if (aFirst) winsB++; else winsA++; }
		else timeouts++;
	}
	return { winsA, winsB, timeouts };
}

// --- Run ---

console.log(`\nSimulating ${GAMES_PER_MATCHUP.toLocaleString()} games per matchup (${MODE} mode)...\n`);

// winRate[a][b] = probability that `a` beats `b`
const winRate: Partial<Record<Level, Partial<Record<Level, number>>>> = {};
for (const lvl of ALL_LEVELS) winRate[lvl] = {};

let totalTimeouts = 0;

for (let i = 0; i < ALL_LEVELS.length; i++) {
	for (let j = i + 1; j < ALL_LEVELS.length; j++) {
		const a = ALL_LEVELS[i];
		const b = ALL_LEVELS[j];
		if (a === undefined || b === undefined) continue;

		const { winsA, winsB, timeouts } = runMatchup(a, b, GAMES_PER_MATCHUP);
		totalTimeouts += timeouts;

		const rA = winsA / GAMES_PER_MATCHUP;
		const rB = winsB / GAMES_PER_MATCHUP;

		winRate[a]![b] = rA;
		winRate[b]![a] = rB;

		const bar = (r: number) => '█'.repeat(Math.round(r * 20)).padEnd(20);
		console.log(`  ${a.padEnd(12)} vs ${b.padEnd(12)}  ${bar(rA)} ${(rA * 100).toFixed(1)}%`);
	}
}

// --- Win rate matrix ---

const COL = 11;
const pct = (v: number | undefined) => (v === undefined ? '—' : `${(v * 100).toFixed(1)}%`);
const pad = (s: string) => s.padStart(COL);

console.log('\n\n── Win Rate Matrix (row beats column) ──────────────────────────────────────────\n');
process.stdout.write(' '.repeat(14));
for (const col of ALL_LEVELS) process.stdout.write(pad(col));
console.log();
console.log('─'.repeat(14 + ALL_LEVELS.length * COL));

for (const row of ALL_LEVELS) {
	process.stdout.write(row.padEnd(14));
	for (const col of ALL_LEVELS) {
		if (row === col) process.stdout.write(pad('·'));
		else process.stdout.write(pad(pct(winRate[row]?.[col])));
	}
	console.log();
}

console.log('─'.repeat(14 + ALL_LEVELS.length * COL));

if (totalTimeouts > 0) {
	console.log(`\n⚠  ${totalTimeouts} game(s) hit the ${MAX_TURNS}-turn limit and were excluded from rates.`);
}

// --- Summary: expected ranking ---

console.log('\n── Ranking by average win rate vs all others ───────────────────────────────────\n');

const ranked = ALL_LEVELS.map((lvl) => {
	const opponents = ALL_LEVELS.filter((o) => o !== lvl);
	const avg =
		opponents.reduce((sum, opp) => sum + (winRate[lvl]?.[opp] ?? 0.5), 0) / opponents.length;
	return { lvl, avg };
}).sort((a, b) => b.avg - a.avg);

for (const { lvl, avg } of ranked) {
	const bar = '█'.repeat(Math.round(avg * 30)).padEnd(30);
	console.log(`  ${lvl.padEnd(12)}  ${bar}  ${(avg * 100).toFixed(1)}%`);
}

console.log();

// --- 4-player tournament ---

const combos4p = combinations(ALL_LEVELS, 4);
const totalGames4p = combos4p.length * GAMES_PER_4P_COMBO;
console.log(
	`\nSimulating ${GAMES_PER_4P_COMBO} × ${combos4p.length} combos = ${totalGames4p.toLocaleString()} 4-player games (${MODE} mode)...\n`
);

const wins4p: Partial<Record<Level, number>> = {};
const plays4p: Partial<Record<Level, number>> = {};
for (const lvl of ALL_LEVELS) {
	wins4p[lvl] = 0;
	plays4p[lvl] = 0;
}
let timeouts4p = 0;

for (const combo of combos4p) {
	for (let i = 0; i < GAMES_PER_4P_COMBO; i++) {
		// Shuffle seating each game to eliminate first-player bias
		const seated = shuffle(combo);
		const winnerIdx = simulateNPlayerGame(seated);
		if (winnerIdx === null) {
			timeouts4p++;
		} else {
			for (const lvl of combo) plays4p[lvl] = (plays4p[lvl] ?? 0) + 1;
			const winnerLevel = seated[winnerIdx];
			if (winnerLevel !== undefined) wins4p[winnerLevel] = (wins4p[winnerLevel] ?? 0) + 1;
		}
	}
}

console.log('── 4-player win rates (random baseline: 25%) ───────────────────────────────────\n');

const ranked4p = ALL_LEVELS.map((lvl) => ({
	lvl,
	rate: (wins4p[lvl] ?? 0) / Math.max(1, plays4p[lvl] ?? 0)
})).sort((a, b) => b.rate - a.rate);

for (const { lvl, rate } of ranked4p) {
	const bar = '█'.repeat(Math.round(rate * 80)).padEnd(20);
	console.log(`  ${lvl.padEnd(12)}  ${bar}  ${(rate * 100).toFixed(1)}%`);
}

if (timeouts4p > 0) {
	console.log(`\n⚠  ${timeouts4p} 4-player game(s) hit the ${MAX_TURNS}-turn limit and were excluded.`);
}

console.log();
