// Procedural sound effects via the Web Audio API — no external assets.
// A small synth toolkit + a named-SFX map. Everything is generated from
// oscillators and noise so the whole set ships as code, not files.

let ctx: AudioContext | null = null;
let master: GainNode | null = null;
let muted = false;

const MUTE_KEY = 'whot-muted';

type Win = Window & { webkitAudioContext?: typeof AudioContext };

function ensure(): AudioContext | null {
	if (typeof window === 'undefined') return null;
	if (!ctx) {
		const AC = window.AudioContext ?? (window as Win).webkitAudioContext;
		if (!AC) return null;
		ctx = new AC();
		master = ctx.createGain();
		master.gain.value = 0.85;
		master.connect(ctx.destination);
	}
	if (ctx.state === 'suspended') void ctx.resume();
	return ctx;
}

/** Prime the audio graph on the first user gesture (browsers block autoplay). */
export function initSound(): void {
	if (typeof window === 'undefined') return;
	muted = localStorage.getItem(MUTE_KEY) === '1';
	const unlock = () => ensure();
	window.addEventListener('pointerdown', unlock, { once: true });
	window.addEventListener('keydown', unlock, { once: true });
}

export function isMuted(): boolean {
	return muted;
}
export function setMuted(v: boolean): void {
	muted = v;
	if (typeof window !== 'undefined') localStorage.setItem(MUTE_KEY, v ? '1' : '0');
}

// --- primitives ---

function shape(g: GainNode, t: number, peak: number, dur: number, attack = 0.005): void {
	g.gain.setValueAtTime(0.0001, t);
	g.gain.exponentialRampToValueAtTime(peak, t + attack);
	g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
}

type ToneOpts = {
	type?: OscillatorType;
	peak?: number;
	at?: number;
	glideTo?: number;
	pan?: number;
};

function tone(freq: number, dur: number, opts: ToneOpts = {}): void {
	const c = ensure();
	if (!c || !master) return;
	const { type = 'sine', peak = 0.3, at = 0, glideTo, pan = 0 } = opts;
	const t = c.currentTime + at;
	const o = c.createOscillator();
	const g = c.createGain();
	o.type = type;
	o.frequency.setValueAtTime(freq, t);
	if (glideTo) o.frequency.exponentialRampToValueAtTime(glideTo, t + dur);
	shape(g, t, peak, dur);
	o.connect(g);
	if (pan && c.createStereoPanner) {
		const p = c.createStereoPanner();
		p.pan.value = pan;
		g.connect(p);
		p.connect(master);
	} else {
		g.connect(master);
	}
	o.start(t);
	o.stop(t + dur + 0.03);
}

type NoiseOpts = {
	peak?: number;
	at?: number;
	type?: BiquadFilterType;
	freq?: number;
	glideTo?: number;
};

function noise(dur: number, opts: NoiseOpts = {}): void {
	const c = ensure();
	if (!c || !master) return;
	const { peak = 0.3, at = 0, type = 'highpass', freq = 1000, glideTo } = opts;
	const t = c.currentTime + at;
	const len = Math.max(1, Math.floor(c.sampleRate * dur));
	const buf = c.createBuffer(1, len, c.sampleRate);
	const d = buf.getChannelData(0);
	for (let i = 0; i < len; i++) d[i] = Math.random() * 2 - 1;
	const src = c.createBufferSource();
	src.buffer = buf;
	const f = c.createBiquadFilter();
	f.type = type;
	f.frequency.setValueAtTime(freq, t);
	if (glideTo) f.frequency.exponentialRampToValueAtTime(glideTo, t + dur);
	const g = c.createGain();
	shape(g, t, peak, dur);
	src.connect(f);
	f.connect(g);
	g.connect(master);
	src.start(t);
	src.stop(t + dur + 0.03);
}

// --- named effects (punchy, arcade-leaning) ---

export type SoundName =
	| 'click'
	| 'deal'
	| 'play'
	| 'draw'
	| 'pick2'
	| 'pick3'
	| 'youhit'
	| 'skip'
	| 'market'
	| 'whot'
	| 'holdon'
	| 'lastcard'
	| 'win'
	| 'lose'
	| 'invalid';

const SFX: Record<SoundName, () => void> = {
	click: () => tone(520, 0.06, { type: 'square', peak: 0.1 }),
	deal: () => noise(0.08, { peak: 0.16, type: 'highpass', freq: 1200 }),
	play: () => {
		tone(420, 0.1, { type: 'triangle', peak: 0.24, glideTo: 210 });
		noise(0.05, { peak: 0.1, type: 'highpass', freq: 2200 });
	},
	draw: () => noise(0.18, { peak: 0.14, type: 'bandpass', freq: 600, glideTo: 1500 }),
	pick2: () => {
		tone(200, 0.14, { type: 'square', peak: 0.26, glideTo: 120 });
		tone(150, 0.16, { type: 'sine', peak: 0.2, at: 0.08, glideTo: 90 });
	},
	pick3: () => {
		tone(180, 0.15, { type: 'square', peak: 0.28, glideTo: 100 });
		tone(140, 0.16, { type: 'square', peak: 0.24, at: 0.09, glideTo: 90 });
		tone(100, 0.2, { type: 'sine', peak: 0.2, at: 0.18, glideTo: 60 });
	},
	youhit: () => {
		tone(90, 0.3, { type: 'sawtooth', peak: 0.28, glideTo: 55 });
		noise(0.24, { peak: 0.18, type: 'lowpass', freq: 420 });
	},
	skip: () => noise(0.28, { peak: 0.2, type: 'bandpass', freq: 400, glideTo: 3200 }),
	market: () =>
		[0, 1, 2, 3].forEach((i) =>
			tone(300 + i * 130, 0.12, { type: 'triangle', peak: 0.16, at: i * 0.06 })
		),
	whot: () =>
		[0, 3, 5, 8, 12].forEach((s, i) =>
			tone(523 * Math.pow(2, s / 12), 0.5 - i * 0.05, { type: 'sine', peak: 0.15, at: i * 0.05 })
		),
	holdon: () => {
		tone(440, 0.1, { type: 'triangle', peak: 0.22 });
		tone(660, 0.14, { type: 'triangle', peak: 0.22, at: 0.1 });
	},
	lastcard: () => {
		tone(1046, 0.1, { type: 'square', peak: 0.22 });
		tone(1046, 0.12, { type: 'square', peak: 0.22, at: 0.16 });
	},
	win: () =>
		[523, 659, 784, 1046].forEach((f, i) =>
			tone(f, 0.55 - i * 0.05, { type: 'triangle', peak: 0.24, at: i * 0.09 })
		),
	lose: () =>
		[392, 330, 262].forEach((f, i) =>
			tone(f, 0.4, { type: 'sine', peak: 0.22, at: i * 0.14, glideTo: f * 0.9 })
		),
	invalid: () => tone(160, 0.18, { type: 'sawtooth', peak: 0.2, glideTo: 110 })
};

export function play(name: SoundName): void {
	if (muted) return;
	try {
		SFX[name]?.();
	} catch {
		// audio is a nice-to-have; never let it break gameplay
	}
}

/** Rising "counter" blip — pitch climbs with the accumulated stack total. */
export function playStack(total: number): void {
	if (muted) return;
	const base = 260 + Math.min(total, 16) * 45;
	try {
		tone(base, 0.13, { type: 'square', peak: 0.24, glideTo: base * 1.5 });
	} catch {
		/* ignore */
	}
}

/** A short riffle of ticks for the opening deal. */
export function playDeal(n = 6): void {
	if (muted) return;
	for (let i = 0; i < n; i++) {
		try {
			noise(0.05, { peak: 0.12, type: 'highpass', freq: 1300, at: i * 0.07 });
		} catch {
			/* ignore */
		}
	}
}
