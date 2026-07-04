import { browser } from '$app/environment';

type Level = 'good' | 'ok' | 'poor';

/**
 * Network-quality readings via Cloudflare's speedtest module. A full measurement
 * (latency + throughput) is used in the menu; in-game we poll *latency only*
 * (tiny packets) so it never disturbs the connection it's measuring.
 */
class NetQuality {
	latency = $state<number | null>(null); // ms
	jitter = $state<number | null>(null); // ms
	down = $state<number | null>(null); // Mbps
	up = $state<number | null>(null); // Mbps
	running = $state(false);

	#engine: { pause(): void; play(): void } | null = null;
	#poll: ReturnType<typeof setInterval> | null = null;

	async measure(full = false): Promise<void> {
		if (!browser || this.running) return;
		this.running = true;
		try {
			const { default: SpeedTest } = await import('@cloudflare/speedtest');
			const measurements = full
				? [
						{ type: 'latency' as const, numPackets: 20 },
						{ type: 'download' as const, bytes: 1e6, count: 6 },
						{ type: 'upload' as const, bytes: 1e6, count: 4 }
					]
				: [{ type: 'latency' as const, numPackets: 10 }];
			const engine = new SpeedTest({ autoStart: false, measurements });
			this.#engine = engine;
			const apply = () => {
				const s = engine.results.getSummary();
				if (s.latency != null) this.latency = Math.round(s.latency);
				if (s.jitter != null) this.jitter = Math.round(s.jitter);
				if (s.download != null) this.down = +(s.download / 1e6).toFixed(1);
				if (s.upload != null) this.up = +(s.upload / 1e6).toFixed(1);
			};
			engine.onResultsChange = apply;
			engine.onFinish = () => {
				apply();
				this.running = false;
				this.#engine = null;
			};
			engine.play();
		} catch {
			this.running = false;
			this.#engine = null;
		}
	}

	/** In-game: refresh latency in the background on an interval. */
	startPolling(intervalMs = 20000): void {
		if (!browser || this.#poll) return;
		void this.measure(false);
		this.#poll = setInterval(() => void this.measure(false), intervalMs);
	}

	stopPolling(): void {
		if (this.#poll) {
			clearInterval(this.#poll);
			this.#poll = null;
		}
		this.#engine?.pause();
		this.#engine = null;
		this.running = false;
	}

	/** green < 80 ms, yellow < 180 ms, orange otherwise (latency-driven). */
	get level(): Level | null {
		const l = this.latency;
		if (l == null) return null;
		if (l < 80) return 'good';
		if (l < 180) return 'ok';
		return 'poor';
	}
}

export const net = new NetQuality();
