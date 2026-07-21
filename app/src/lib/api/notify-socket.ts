import type { Notification } from './types';

/**
 * WebSocket to `/api/ws/notify` for real-time notifications (game invites,
 * accepts, declines). Flushes unread on connect, then streams new ones.
 */
/** Consecutive failed attempts before we give up until the next `connect()`. */
const MAX_ATTEMPTS = 10;

export class NotifySocket {
	#ws: WebSocket | null = null;
	#token: () => string | null;
	#refreshToken: () => Promise<boolean>;
	#onNotification: (n: Notification) => void;
	#closed = false;
	#reconnects = 0;
	#refreshFirst = false;

	constructor(
		/** Lazy, so reconnects don't replay an expired 15-minute access token. */
		token: () => string | null,
		refreshToken: () => Promise<boolean>,
		onNotification: (n: Notification) => void
	) {
		this.#token = token;
		this.#refreshToken = refreshToken;
		this.#onNotification = onNotification;
	}

	connect(): void {
		this.#closed = false;
		this.#reconnects = 0;
		void this.#open();
	}

	async #open(): Promise<void> {
		if (this.#closed) return;

		let token = this.#token();
		// Never opened last time → the handshake was rejected, most likely on an
		// expired token. Get a fresh one instead of looping on the dead one.
		if (!token || this.#refreshFirst) {
			this.#refreshFirst = false;
			if (await this.#refreshToken()) token = this.#token();
			if (this.#closed) return;
		}
		if (!token) return;

		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		const ws = new WebSocket(
			`${proto}://${location.host}/api/ws/notify?token=${encodeURIComponent(token)}`
		);
		this.#ws = ws;
		let opened = false;
		ws.onopen = () => {
			opened = true;
			this.#reconnects = 0;
		};
		ws.onmessage = (e) => {
			try {
				this.#onNotification(JSON.parse(e.data as string) as Notification);
			} catch {
				/* ignore malformed */
			}
		};
		ws.onclose = () => {
			if (this.#closed) return;
			if (!opened) this.#refreshFirst = true;
			if (this.#reconnects >= MAX_ATTEMPTS) return;
			const delay = Math.min(1000 * 2 ** this.#reconnects, 10000);
			this.#reconnects += 1;
			setTimeout(() => !this.#closed && void this.#open(), delay);
		};
	}

	close(): void {
		this.#closed = true;
		this.#ws?.close();
		this.#ws = null;
	}
}
