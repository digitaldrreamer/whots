import type { Notification } from './types';

/**
 * WebSocket to `/api/ws/notify` for real-time notifications (game invites,
 * accepts, declines). Flushes unread on connect, then streams new ones.
 */
export class NotifySocket {
	#ws: WebSocket | null = null;
	#token: string;
	#onNotification: (n: Notification) => void;
	#closed = false;
	#reconnects = 0;

	constructor(token: string, onNotification: (n: Notification) => void) {
		this.#token = token;
		this.#onNotification = onNotification;
	}

	connect(): void {
		this.#closed = false;
		this.#open();
	}

	#open(): void {
		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		const ws = new WebSocket(
			`${proto}://${location.host}/api/ws/notify?token=${encodeURIComponent(this.#token)}`
		);
		this.#ws = ws;
		ws.onopen = () => (this.#reconnects = 0);
		ws.onmessage = (e) => {
			try {
				this.#onNotification(JSON.parse(e.data as string) as Notification);
			} catch {
				/* ignore malformed */
			}
		};
		ws.onclose = () => {
			if (this.#closed) return;
			const delay = Math.min(1000 * 2 ** this.#reconnects, 10000);
			this.#reconnects += 1;
			setTimeout(() => !this.#closed && this.#open(), delay);
		};
	}

	close(): void {
		this.#closed = true;
		this.#ws?.close();
		this.#ws = null;
	}
}
