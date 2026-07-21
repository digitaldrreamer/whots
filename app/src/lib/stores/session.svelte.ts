import { create, get, supported } from '@github/webauthn-json';
import type { PublicUser, Session } from '$lib/api/types';

export type SessionStatus = 'loading' | 'authed' | 'anon';

/**
 * Browser-side auth state. The access token lives in memory only; the refresh
 * token is an httpOnly cookie handled by the /auth/* SvelteKit endpoints.
 */
class SessionStore {
	user = $state<PublicUser | null>(null);
	status = $state<SessionStatus>('loading');
	#accessToken: string | null = null;
	#refreshing: Promise<boolean> | null = null;

	get accessToken(): string | null {
		return this.#accessToken;
	}

	#apply(s: Session) {
		this.user = s.user;
		this.#accessToken = s.access_token;
		this.status = 'authed';
	}

	#clear() {
		this.user = null;
		this.#accessToken = null;
		this.status = 'anon';
	}

	/** Restore a session from the refresh cookie on app start. */
	async restore(): Promise<boolean> {
		const ok = await this.#refresh();
		if (!ok) this.status = 'anon';
		return ok;
	}

	/** Re-fetch the current user (e.g. after earning a badge) without touching tokens. */
	async refreshUser(): Promise<void> {
		try {
			const res = await this.apiFetch('/api/users/me');
			if (res.ok) this.user = (await res.json()) as PublicUser;
		} catch {
			/* transient — keep current user */
		}
	}

	async guest(username: string): Promise<void> {
		await this.#authCall('/auth/guest', { username });
	}

	async login(identifier: string, password: string): Promise<void> {
		await this.#authCall('/auth/login', { identifier, password });
	}

	async register(username: string, email: string, password: string): Promise<void> {
		await this.#authCall('/auth/register', { username, email, password });
	}

	/** Claim a guest account: add email + password (keeps username, friends, id). */
	async upgradeGuest(email: string, password: string): Promise<void> {
		const res = await this.apiFetch('/api/auth/upgrade', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ email, password })
		});
		const data = await res.json().catch(() => ({}));
		if (!res.ok) throw new ApiError(res.status, data?.error ?? data?.message ?? 'Upgrade failed');
		this.user = data as PublicUser;
	}

	/** Whether this browser can do WebAuthn passkeys. */
	get passkeysSupported(): boolean {
		return typeof window !== 'undefined' && supported();
	}

	/** Add a passkey to the current account (claims a guest — no email needed). */
	async addPasskey(): Promise<void> {
		const startRes = await this.apiFetch('/api/auth/passkey/register/start', { method: 'POST' });
		if (!startRes.ok) {
			const d = await startRes.json().catch(() => ({}));
			throw new ApiError(startRes.status, d?.error ?? 'Could not start passkey setup');
		}
		const options = await startRes.json();
		const credential = await create(options); // biometric / security-key prompt
		const finRes = await this.apiFetch('/api/auth/passkey/register/finish', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(credential)
		});
		if (!finRes.ok) {
			const d = await finRes.json().catch(() => ({}));
			throw new ApiError(finRes.status, d?.error ?? 'Could not save passkey');
		}
		if (this.user) this.user = { ...this.user, is_guest: false, has_passkey: true };
	}

	/** Sign in with a passkey. `login/start` is public; `finish` goes through the
	 * SvelteKit route so the refresh cookie is set. */
	async loginWithPasskey(username: string): Promise<void> {
		const startRes = await fetch('/api/auth/passkey/login/start', {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ username })
		});
		if (!startRes.ok) {
			const d = await startRes.json().catch(() => ({}));
			throw new ApiError(startRes.status, d?.error ?? 'No passkey for that account');
		}
		const options = await startRes.json();
		const credential = await get(options); // biometric prompt
		await this.#authCall('/auth/passkey-login', { username, credential });
	}

	async logout(): Promise<void> {
		const headers: Record<string, string> = {};
		if (this.#accessToken) headers.authorization = `Bearer ${this.#accessToken}`;
		await fetch('/auth/logout', { method: 'POST', headers }).catch(() => undefined);
		this.#clear();
	}

	async #authCall(path: string, body: unknown): Promise<void> {
		const res = await fetch(path, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		});
		const data = await res.json().catch(() => ({}));
		if (!res.ok) throw new ApiError(res.status, data?.error ?? data?.message ?? 'Request failed');
		this.#apply(data as Session);
	}

	/**
	 * Mint a fresh access token from the refresh cookie. WebSockets need this
	 * explicitly: they authenticate once at the handshake and can't retry with a
	 * new token the way `apiFetch` does, so a socket that outlives the 15-minute
	 * access token has to ask for a new one before reconnecting.
	 */
	refreshAccessToken(): Promise<boolean> {
		return this.#refresh();
	}

	/** Single-flight refresh so concurrent 401s don't stampede the endpoint. */
	#refresh(): Promise<boolean> {
		if (this.#refreshing) return this.#refreshing;
		this.#refreshing = (async () => {
			try {
				const res = await fetch('/auth/refresh', { method: 'POST' });
				if (!res.ok) {
					this.#clear();
					return false;
				}
				this.#apply((await res.json()) as Session);
				return true;
			} catch {
				return false;
			} finally {
				this.#refreshing = null;
			}
		})();
		return this.#refreshing;
	}

	/**
	 * Authenticated fetch against the backend (`/api/...`). Attaches the bearer
	 * token and transparently refreshes + retries once on a 401.
	 */
	async apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
		const doFetch = () =>
			fetch(path, {
				...init,
				headers: {
					...(init.headers as Record<string, string> | undefined),
					...(this.#accessToken ? { authorization: `Bearer ${this.#accessToken}` } : {})
				}
			});

		let res = await doFetch();
		if (res.status === 401 && (await this.#refresh())) {
			res = await doFetch();
		}
		return res;
	}
}

export class ApiError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}
}

export const session = new SessionStore();
