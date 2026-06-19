// --- Shapes ---

export const SHAPES = ['circle', 'triangle', 'cross', 'square', 'star'] as const;
export type Shape = (typeof SHAPES)[number];

// --- Card values ---

export const SUIT_VALUES = [1, 2, 3, 4, 5, 7, 8, 10, 11, 12, 13, 14] as const;
export type SuitValue = (typeof SUIT_VALUES)[number];

export const ACTION_VALUES = [1, 2, 5, 8, 14] as const;
export type ActionValue = (typeof ACTION_VALUES)[number];

export const WHOT_VALUE = 20 as const;
export type WhotValue = typeof WHOT_VALUE;

// --- Cards ---

export type SuitCard = {
	readonly kind: 'suit';
	readonly shape: Shape;
	readonly value: SuitValue;
};

export type WhotCard = {
	readonly kind: 'whot';
	readonly value: WhotValue;
};

export type Card = SuitCard | WhotCard;

// A Whot card after being played — carries the shape the player called.
// Not a Card subtype; only lives on TopCard.
export type PlayedWhot = {
	readonly kind: 'whot';
	readonly value: WhotValue;
	readonly calledShape: Shape;
};

// The effective top of the discard pile used for move validation
export type TopCard = SuitCard | PlayedWhot;

// --- Game mode ---

export type GameMode = 'stack' | 'no-stack';

// --- Action effects ---

export type HoldOnEffect = { readonly kind: 'hold_on' };
export type PickTwoEffect = { readonly kind: 'pick_two' };
export type PickThreeEffect = { readonly kind: 'pick_three' };
export type SuspensionEffect = { readonly kind: 'suspension' };
export type GeneralMarketEffect = { readonly kind: 'general_market' };
export type WhotEffect = { readonly kind: 'whot'; readonly calledShape: Shape };

export type ActionEffect =
	| HoldOnEffect
	| PickTwoEffect
	| PickThreeEffect
	| SuspensionEffect
	| GeneralMarketEffect
	| WhotEffect;

// --- Pending effects (accumulate during stacking) ---

export type PendingPickEffect = {
	readonly kind: 'pick';
	readonly total: number;
};

export type PendingSkipEffect = {
	readonly kind: 'skip';
};

export type PendingEffect = PendingPickEffect | PendingSkipEffect;

// --- Players ---

// Branded so a raw string can never be passed where a PlayerId is expected.
// The single allowed coercion is in createPlayerId() in state.ts.
export type PlayerId = string & { readonly _brand: 'PlayerId' };

export type HumanPlayer = {
	readonly id: PlayerId;
	readonly kind: 'human';
	readonly name: string;
	hand: Card[];
};

export const DIFFICULTIES = [
	'pikin',
	'smallz',
	'isabiSmall',
	'chief',
	'egbon',
	'jagaban'
] as const;
export type Difficulty = (typeof DIFFICULTIES)[number];

export type ComputerPlayer = {
	readonly id: PlayerId;
	readonly kind: 'computer';
	readonly name: string;
	readonly difficulty: Difficulty;
	hand: Card[];
};

export type TeeNoblePlayer = {
	readonly id: PlayerId;
	readonly kind: 'tee-noble';
	readonly name: string;
	hand: Card[];
};

export type Player = HumanPlayer | ComputerPlayer | TeeNoblePlayer;

// --- Reasoning modules ---

export const REASONING_MODULES = [
	'hand-thinning',
	'action-awareness',
	'threat-detection',
	'card-probability',
	'whot-intelligence',
	'anticipation',
	'setup-plays'
] as const;
export type ReasoningModule = (typeof REASONING_MODULES)[number];

export const DIFFICULTY_MODULES: Record<Difficulty, readonly ReasoningModule[]> = {
	pikin: [],
	smallz: ['hand-thinning'],
	isabiSmall: ['hand-thinning', 'action-awareness'],
	chief: ['hand-thinning', 'action-awareness', 'threat-detection'],
	egbon: [
		'hand-thinning',
		'action-awareness',
		'threat-detection',
		'card-probability',
		'whot-intelligence',
		'setup-plays'
	],
	jagaban: [
		'hand-thinning',
		'action-awareness',
		'threat-detection',
		'card-probability',
		'whot-intelligence',
		'setup-plays',
		'anticipation'
	]
};

// --- Game state ---

export type GamePhase = 'playing' | 'finished';

export type GameState = {
	readonly id: string;
	readonly mode: GameMode;
	players: Player[];
	stockPile: Card[];
	discardPile: Card[];
	topCard: TopCard;
	currentPlayerIndex: number;
	phase: GamePhase;
	pendingEffect: PendingEffect | null;
	winner: Player | null;
};
