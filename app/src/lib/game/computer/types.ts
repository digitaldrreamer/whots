import type { GameState, Shape, SuitCard } from '../types.js';

// The full set of actions the computer can take

export type SuitCandidate = {
	readonly kind: 'play-suit';
	readonly card: SuitCard;
};

export type WhotCandidate = {
	readonly kind: 'play-whot';
	readonly calledShape: Shape;
};

export type DrawCandidate = {
	readonly kind: 'draw';
};

export type Candidate = SuitCandidate | WhotCandidate | DrawCandidate;

// Everything a module needs to score a candidate
export type ModuleContext = {
	readonly state: GameState;
	readonly playerIndex: number;
	readonly candidates: readonly Candidate[];
	// Sizes of each opponent's hand (-1 for the acting player's own slot)
	readonly opponentHandSizes: readonly number[];
	// Estimated cards of each shape still in play (opponents + stock), derived from
	// discard pile + own hand — the only public information available
	readonly shapeRemaining: Readonly<Record<Shape, number>>;
};

// A module scores a single candidate given the full context.
// Higher score = more desirable.
export type ScoringModule = (candidate: Candidate, ctx: ModuleContext) => number;
