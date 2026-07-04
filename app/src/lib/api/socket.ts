import type { ClientEvent, ServerEvent } from './types';

export type SocketStatus = 'connecting' | 'open' | 'closed' | 'error';

interface GameSocketOptions {
	gameId: string;
	token: string;
	onEvent: (ev: ServerEvent) => void;
	onStatus?: (status: SocketStatus) => void;
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

	constructor(opts: GameSocketOptions) {
		this.#opts = opts;
	}

	connect(): void {
		this.#closed = false;
		this.#open();
	}

	#open(): void {
		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		const url = `${proto}://${location.host}/api/ws/game/${this.#opts.gameId}?token=${encodeURIComponent(this.#opts.token)}`;
		this.#opts.onStatus?.('connecting');

		const ws = new WebSocket(url);
		this.#ws = ws;

		ws.onopen = () => {
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

		ws.onclose = () => {
			this.#opts.onStatus?.('closed');
			if (this.#closed) return;
			// Backoff reconnect (server keeps game state in Redis).
			const delay = Math.min(1000 * 2 ** this.#reconnects, 8000);
			this.#reconnects += 1;
			setTimeout(() => {
				if (!this.#closed) this.#open();
			}, delay);
		};
	}

	send(ev: ClientEvent): void {
		if (this.#ws?.readyState === WebSocket.OPEN) {
			this.#ws.send(JSON.stringify(ev));
		}
	}

	close(): void {
		this.#closed = true;
		this.#ws?.close();
		this.#ws = null;
	}
}
