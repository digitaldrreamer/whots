import { ACTION_VALUES } from './types.js';
import type { ActionValue, Card, PlayedWhot, SuitCard, TopCard, WhotCard } from './types.js';

export function isSuitCard(card: Card): card is SuitCard {
	return card.kind === 'suit';
}

export function isWhotCard(card: Card): card is WhotCard {
	return card.kind === 'whot';
}

export function isPlayedWhot(top: TopCard): top is PlayedWhot {
	return top.kind === 'whot';
}

export function isActionValue(value: number): value is ActionValue {
	return (ACTION_VALUES as ReadonlyArray<number>).includes(value);
}

// Narrows a SuitCard to one that carries an action effect
export function isActionCard(card: SuitCard): card is SuitCard & { value: ActionValue } {
	return isActionValue(card.value);
}
