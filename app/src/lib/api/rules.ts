import type { Card, GameMode, PendingEffect, TopCard } from './types';

// Client-side mirror of the server's move legality (server/src/game/moves.rs).
// Used only to highlight/enable playable cards — the server remains authoritative
// and rejects anything illegal.
export function canPlay(
	card: Card,
	top: TopCard,
	pending: PendingEffect | null,
	mode: GameMode
): boolean {
	// Stack mode with a pending pick: only 2s and 5s can counter.
	if (pending?.kind === 'pick' && mode === 'stack') {
		return card.kind === 'suit' && (card.value === 2 || card.value === 5);
	}

	// Whot is always playable outside a counter window.
	if (card.kind === 'whot') return true;

	// card is a suit card
	if (top.kind === 'whot') return card.shape === top.called_shape;
	return card.shape === top.shape || card.value === top.value;
}

/** True if two cards are the same identity (for selection/highlight matching). */
export function sameCard(a: Card, b: Card): boolean {
	if (a.kind === 'whot' && b.kind === 'whot') return true;
	if (a.kind === 'suit' && b.kind === 'suit') return a.shape === b.shape && a.value === b.value;
	return false;
}
