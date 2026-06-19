import type { Card, GameMode, PendingEffect, TopCard } from './types.js';
import { isPlayedWhot, isSuitCard } from './guards.js';

export function canPlay(
	card: Card,
	top: TopCard,
	pending: PendingEffect | null,
	mode: GameMode
): boolean {
	// In stack mode with a pending pick, only 2s and 5s can counter
	if (pending?.kind === 'pick' && mode === 'stack') {
		if (!isSuitCard(card)) return false;
		return card.value === 2 || card.value === 5;
	}

	// Whot is always playable (outside of a pending counter window)
	if (card.kind === 'whot') return true;

	// Top is a played whot: must match the called shape
	if (isPlayedWhot(top)) return card.shape === top.calledShape;

	// Standard: match by shape or number
	return card.shape === top.shape || card.value === top.value;
}

export function getValidMoves(
	hand: Card[],
	top: TopCard,
	pending: PendingEffect | null,
	mode: GameMode
): Card[] {
	return hand.filter((card) => canPlay(card, top, pending, mode));
}
