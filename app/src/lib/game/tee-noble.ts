// --- Challenge status ---

export type ChallengeOutcome = 'won' | 'lost' | 'declined';

export type TeeNobleStatus =
	| { readonly kind: 'idle' }
	| { readonly kind: 'pending' } // offered, awaiting player decision
	| { readonly kind: 'active' } // accepted, game in progress
	| { readonly kind: 'resolved'; readonly outcome: ChallengeOutcome };

// --- Session state ---

// Persisted across games within a session.
// All fields are plain serialisable values so the caller can store them freely.
export type TeeNobleSession = {
	readonly gamesPlayed: number;
	readonly winStreak: number;
	// How many games have passed since the last challenge was triggered.
	// Used to enforce a cooldown between appearances.
	readonly gamesSinceLastChallenge: number;
	// Once a player beats Tee-Noble, re-appearances become extremely rare.
	readonly hasWonBefore: boolean;
	readonly status: TeeNobleStatus;
};

// --- Trigger parameters ---

const BASE_PROBABILITY = 0.05; // 5% per game at zero streak
const STREAK_BONUS = 0.05; // +5% per consecutive win
const MAX_PROBABILITY = 0.30; // hard cap
const COOLDOWN_GAMES = 3; // minimum games between challenge appearances
const REPEAT_WIN_MULTIPLIER = 0.1; // 90% reduction after already winning once

function triggerProbability(session: TeeNobleSession): number {
	const raw = BASE_PROBABILITY + session.winStreak * STREAK_BONUS;
	const capped = Math.min(raw, MAX_PROBABILITY);
	return session.hasWonBefore ? capped * REPEAT_WIN_MULTIPLIER : capped;
}

function shouldTrigger(session: TeeNobleSession): boolean {
	if (session.status.kind === 'pending' || session.status.kind === 'active') return false;
	if (session.gamesSinceLastChallenge < COOLDOWN_GAMES) return false;
	return Math.random() < triggerProbability(session);
}

// --- Public API ---

export function createSession(): TeeNobleSession {
	return {
		gamesPlayed: 0,
		winStreak: 0,
		gamesSinceLastChallenge: 0,
		hasWonBefore: false,
		status: { kind: 'idle' }
	};
}

// Call after every completed game (win or loss against a regular opponent).
// Returns the updated session — may transition status to 'pending' if Tee-Noble appears.
export function afterGame(session: TeeNobleSession, playerWon: boolean): TeeNobleSession {
	const next: TeeNobleSession = {
		...session,
		gamesPlayed: session.gamesPlayed + 1,
		winStreak: playerWon ? session.winStreak + 1 : 0,
		gamesSinceLastChallenge: session.gamesSinceLastChallenge + 1,
		status: { kind: 'idle' }
	};

	if (shouldTrigger(next)) {
		return { ...next, status: { kind: 'pending' }, gamesSinceLastChallenge: 0 };
	}

	return next;
}

// Player accepted the challenge — start the game against Tee-Noble.
export function acceptChallenge(session: TeeNobleSession): TeeNobleSession {
	if (session.status.kind !== 'pending') {
		throw new Error('No pending challenge to accept');
	}
	return { ...session, status: { kind: 'active' } };
}

// Player declined — Tee-Noble disappears until the next random trigger.
export function declineChallenge(session: TeeNobleSession): TeeNobleSession {
	if (session.status.kind !== 'pending') {
		throw new Error('No pending challenge to decline');
	}
	return { ...session, status: { kind: 'resolved', outcome: 'declined' } };
}

// Call when the Tee-Noble game ends.
// 'won' here means the player beat Tee-Noble.
export function resolveChallenge(
	session: TeeNobleSession,
	outcome: 'won' | 'lost'
): TeeNobleSession {
	if (session.status.kind !== 'active') {
		throw new Error('No active challenge to resolve');
	}
	return {
		...session,
		hasWonBefore: outcome === 'won' ? true : session.hasWonBefore,
		// Win streak resets on loss, continues on win
		winStreak: outcome === 'won' ? session.winStreak + 1 : 0,
		status: { kind: 'resolved', outcome }
	};
}

// Convenience predicates — keeps consumers from pattern-matching on status directly

export function isChallengeIdle(session: TeeNobleSession): boolean {
	return session.status.kind === 'idle';
}

export function isChallengePending(session: TeeNobleSession): boolean {
	return session.status.kind === 'pending';
}

export function isChallengeActive(session: TeeNobleSession): boolean {
	return session.status.kind === 'active';
}

export function isChallengeResolved(
	session: TeeNobleSession
): session is TeeNobleSession & { status: { kind: 'resolved'; outcome: ChallengeOutcome } } {
	return session.status.kind === 'resolved';
}
