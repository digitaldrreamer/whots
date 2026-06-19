import { SHAPES, SUIT_VALUES, WHOT_VALUE } from './types.js';
import type { Card, SuitCard, WhotCard } from './types.js';

const WHOT_COUNT = 5;

export function createDeck(): Card[] {
	const suitCards: SuitCard[] = SHAPES.flatMap((shape) =>
		SUIT_VALUES.map((value): SuitCard => ({ kind: 'suit', shape, value }))
	);

	const whotCards: WhotCard[] = Array.from(
		{ length: WHOT_COUNT },
		(): WhotCard => ({ kind: 'whot', value: WHOT_VALUE })
	);

	return [...suitCards, ...whotCards];
}

// Fisher-Yates — O(n), every permutation equally likely
export function shuffle<T>(items: readonly T[]): T[] {
	const result = [...items];
	for (let i = result.length - 1; i > 0; i--) {
		const j = Math.floor(Math.random() * (i + 1));
		const temp = result[i];
		result[i] = result[j];
		result[j] = temp;
	}
	return result;
}

export function createShuffledDeck(): Card[] {
	return shuffle(createDeck());
}
