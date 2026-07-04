import type { ActionEffect, PendingEffect, Shape, SuitCard, WhotEffect } from './types.js';

// Two separate functions — each path is fully typed with no optional params or coercion

export function getSuitCardEffect(card: SuitCard): ActionEffect | null {
	switch (card.value) {
		case 1:
			return { kind: 'hold_on' };
		case 2:
			return { kind: 'pick_two' };
		case 5:
			return { kind: 'pick_three' };
		case 8:
			return { kind: 'suspension' };
		case 14:
			return { kind: 'general_market' };
		default:
			return null;
	}
}

export function getWhotEffect(calledShape: Shape): WhotEffect {
	return { kind: 'whot', calledShape };
}

// Fold a resolved effect into the current pending state.
// pick_two / pick_three accumulate (stack mode); others resolve immediately.
export function accumulate(
	effect: ActionEffect,
	pending: PendingEffect | null
): PendingEffect | null {
	switch (effect.kind) {
		case 'pick_two':
			return { kind: 'pick', total: (pending?.kind === 'pick' ? pending.total : 0) + 2 };
		case 'pick_three':
			return { kind: 'pick', total: (pending?.kind === 'pick' ? pending.total : 0) + 3 };
		case 'hold_on':
		case 'suspension':
			return { kind: 'skip' };
		case 'general_market':
		case 'whot':
			return null;
	}
}
