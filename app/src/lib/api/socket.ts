import type { ClientEvent, ServerEvent } from './types';

export type SocketStatus = 'connecting' | 'open' | 'closed' | 'error';

/** Why we stopped trying. Surfaced so the UI can say something truthful. */
export type SocketFailure = 'auth' | 'gone' | 'unreachable';

/** Server-side close code for "this game no longer exists" — never retry it. */
const CLOSE_GAME_GONE = 4404;

/** Consecutive failed attempts before we stop and tell the user. */
const MAX_ATTEMPTS = 10;

interface GameSocketOptions {
	gameId: string;
	/**
	 * Read lazily, once per attempt. Access tokens expire after 15 minutes, so a
	 * token captured at construction turns every later reconnect into a permanent
	 * 401 loop.
	 */
	token: () => string | null;
	/** Mint a new access token; false if the refresh cookie is dead too. */
	refreshToken: () => Promise<boolean>;
	onEvent: (ev: ServerEvent) => void;
	onStatus?: (status: SocketStatus) => void;
	/** Terminal — no further attempts will be made. */
	onFailure?: (reason: SocketFailure) => void;
}

/**
 * Thin WebSocket client for a single game. Connects to the same-origin
 * `/api/ws/game/:id` (Traefik routes it to the Rust server; in dev the vite
 * proxy forwards it). The access token rides in the query string because
 * browsers can't set headers on a WebSocket handshake.
 */
export class GameSocket {
	#ws: WebSocket | null = null;
	readonly #opts: GameSocketOptions;
	#closed = false;
	#reconnects = 0;
	/** Set when the previous attempt died before opening — see `#open`. */
	#refreshFirst = false;
	/**
	 * Bumped per attempt. `#open` awaits a token refresh partway through, and a
	 * `reconnect()` arriving during that await would otherwise race the pending
	 * attempt and leave two live sockets for one user.
	 */
	#gen = 0;

	constructor(opts: GameSocketOptions) {
		this.#opts = opts;
	}

	connect(): void {
		this.#closed = false;
		void this.#open();
	}

	async #open(): Promise<void> {
		if (this.#closed) return;
		const gen = ++this.#gen;
		this.#opts.onStatus?.('connecting');

		let token = this.#opts.token();
		// A socket that never reached `open` was almost certainly rejected at the
		// handshake, and an expired access token is by far the likeliest reason.
		// Replaying the same dead token would loop forever, so mint a new one.
		if (!token || this.#refreshFirst) {
			this.#refreshFirst = false;
			if (await this.#opts.refreshToken()) token = this.#opts.token();
			if (this.#closed || gen !== this.#gen) return; // superseded while awaiting
		}
		if (!token) {
			this.#fail('auth');
			return;
		}

		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		const url = `${proto}://${location.host}/api/ws/game/${this.#opts.gameId}?token=${encodeURIComponent(token)}`;

		const ws = new WebSocket(url);
		this.#ws = ws;
		let opened = false;

		ws.onopen = () => {
			opened = true;
			this.#reconnects = 0;
			this.#opts.onStatus?.('open');
		};

		ws.onmessage = (e) => {
			let ev: ServerEvent;
			try {
				ev = JSON.parse(e.data as string) as ServerEvent;
			} catch {
				return;
			}
			this.#opts.onEvent(ev);
		};

		ws.onerror = () => this.#opts.onStatus?.('error');

		ws.onclose = (e) => {
			this.#opts.onStatus?.('closed');
			if (this.#closed) return;

			// The game is gone for good — retrying can only fail.
			if (e.code === CLOSE_GAME_GONE) {
				this.#fail('gone');
				return;
			}
			if (!opened) this.#refreshFirst = true;
			if (this.#reconnects >= MAX_ATTEMPTS) {
				this.#fail('unreachable');
				return;
			}

			// Backoff reconnect (server keeps game state in Redis).
			const delay = Math.min(1000 * 2 ** this.#reconnects, 8000);
			this.#reconnects += 1;
			setTimeout(() => {
				if (!this.#closed) void this.#open();
			}, delay);
		};
	}

	/** Stop for good and report why. */
	#fail(reason: SocketFailure): void {
		this.#closed = true;
		this.#ws = null;
		this.#opts.onFailure?.(reason);
	}

	send(ev: ClientEvent): void {
		if (this.#ws?.readyState === WebSocket.OPEN) {
			this.#ws.send(JSON.stringify(ev));
		}
	}

	/**
	 * Force a fresh socket: drop the current one (discarding any buffered sends
	 * that would otherwise replay later) and reopen. The server re-sends the
	 * authoritative game state on connect, so the client re-syncs cleanly.
	 */
	reconnect(): void {
		if (this.#closed) return;
		const ws = this.#ws;
		this.#ws = null;
		if (ws) {
			ws.onclose = null; // don't let the old socket schedule its own reconnect
			ws.onerror = null;
			ws.onmessage = null;
			try {
				ws.close();
			} catch {
				/* already closing */
			}
		}
		this.#reconnects = 0;
		void this.#open();
	}

	close(): void {
		this.#closed = true;
		this.#ws?.close();
		this.#ws = null;
	}
}
