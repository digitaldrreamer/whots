// Wire types mirroring the Rust backend (serde representations).
// Keep in sync with server/src/game/types.ts and server/src/routes/{auth,games,ws}.rs.

export const SHAPES = ['circle', 'triangle', 'cross', 'square', 'star'] as const;
export type Shape = (typeof SHAPES)[number];

export type Card = { kind: 'suit'; shape: Shape; value: number } | { kind: 'whot' };

export type TopCard =
	| { kind: 'suit'; shape: Shape; value: number }
	| { kind: 'whot'; called_shape: Shape };

export type GameMode = 'stack' | 'no_stack';

export type Difficulty =
	| 'pikin'
	| 'smallz'
	| 'isabi_small'
	| 'chief'
	| 'egbon'
	| 'jagaban'
	| 'tee_noble';

export type GamePhase = 'playing' | 'finished';

// `count` = number of penalty cards owed; `card` = the number that started it
// (2 or 5). Cards actually drawn = count * (card === 5 ? 3 : 2).
export type PendingEffect = { kind: 'pick'; count: number; card: number } | { kind: 'skip' };

export type SeatKind =
	| { kind: 'human'; user_id: string }
	| { kind: 'ai'; difficulty: Difficulty };

export interface SeatView {
	name: string;
	kind: SeatKind;
	hand: Card[]; // populated only for the viewing player
	hand_size: number; // always the true count
	owed_draws: number; // General Market cards this seat still has to draw
}

export interface GameStateView {
	id: string;
	mode: GameMode;
	seats: SeatView[];
	stock_size: number;
	discard_top: TopCard;
	current_seat_index: number;
	phase: GamePhase;
	pending_effect: PendingEffect | null;
	winner_index: number | null;
}

// ── Game creation (POST /api/games) ──────────────────────────────────────────────
export type SeatSpec =
	| { kind: 'human'; user_id: string }
	| { kind: 'ai'; difficulty: Difficulty; name: string };

export interface CreateGameRequest {
	mode: GameMode;
	seats: SeatSpec[];
}

export interface CreateGameResponse {
	game_id: string;
}

// ── WebSocket wire messages ───────────────────────────────────────────────────────
// Client -> server: the card action carried inside `play_card`.
export type WsAction =
	| { kind: 'suit'; shape: Shape; value: number }
	| { kind: 'whot'; called_shape: Shape };

export type ClientEvent =
	| { type: 'play_card'; action: WsAction }
	| { type: 'play_stack'; value: number; shapes: Shape[] }
	| { type: 'draw' }
	| { type: 'chat_message'; text: string }
	| { type: 'rtc_offer'; to: string; sdp: string }
	| { type: 'rtc_answer'; to: string; sdp: string }
	| { type: 'rtc_ice'; to: string; candidate: string };

export type ServerEvent =
	| { type: 'game_state'; state: GameStateView }
	| { type: 'game_over'; winner_index: number | null; winner_name: string | null }
	| { type: 'error'; message: string }
	| { type: 'rtc_signal'; from: string; kind: string; payload: string }
	| { type: 'chat_message'; from: string; text: string };

// ── Auth ──────────────────────────────────────────────────────────────────────────
export interface PublicUser {
	id: string;
	username: string;
	display_name: string;
	avatar_url: string | null;
	is_guest: boolean;
}

// What the browser receives from our SvelteKit auth endpoints. The refresh token
// never reaches the browser — it lives in an httpOnly cookie.
export interface Session {
	user: PublicUser;
	access_token: string;
}

// ── Social / multiplayer ────────────────────────────────────────────────────────
export interface FriendRow {
	id: string;
	username: string;
	display_name: string;
	avatar_url: string | null;
	since: string;
}

export interface Notification {
	id: string;
	user_id: string;
	kind: 'game_invite' | 'game_accepted' | 'game_declined' | string;
	payload: { game_id?: string; from_username?: string; [k: string]: unknown };
	read: boolean;
	created_at: string;
}

export interface MatchmakingJoinResponse {
	matched: boolean;
	game_id: string | null;
}

export interface MatchmakingStatus {
	in_queue: boolean;
	mode: string | null;
}
