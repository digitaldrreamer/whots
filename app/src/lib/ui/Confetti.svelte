<script lang="ts">
	// Lightweight canvas confetti. Fires a burst whenever `trigger` changes.
	let { trigger = 0 }: { trigger?: number } = $props();

	const COLORS = ['#e8b84b', '#d64545', '#2f9e6f', '#2f6fed', '#8b5cf6', '#f6c453'];

	type P = {
		x: number;
		y: number;
		vx: number;
		vy: number;
		rot: number;
		vr: number;
		w: number;
		h: number;
		c: string;
	};

	let canvas: HTMLCanvasElement;
	let raf = 0;
	let last = 0;

	function burst() {
		if (!canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		const dpr = Math.min(window.devicePixelRatio || 1, 2);
		const W = window.innerWidth;
		const H = window.innerHeight;
		canvas.width = W * dpr;
		canvas.height = H * dpr;
		ctx.scale(dpr, dpr);

		const parts: P[] = [];
		const n = 150;
		for (let i = 0; i < n; i++) {
			const fromLeft = i < n / 2;
			parts.push({
				x: fromLeft ? W * 0.15 : W * 0.85,
				y: H * 0.35,
				vx: (fromLeft ? 1 : -1) * (2 + Math.random() * 7),
				vy: -(6 + Math.random() * 9),
				rot: Math.random() * Math.PI,
				vr: (Math.random() - 0.5) * 0.4,
				w: 6 + Math.random() * 6,
				h: 8 + Math.random() * 8,
				c: COLORS[(Math.random() * COLORS.length) | 0]
			});
		}

		cancelAnimationFrame(raf);
		last = performance.now();
		const gravity = 26;

		const frame = (now: number) => {
			const dt = Math.min(0.05, (now - last) / 1000);
			last = now;
			ctx.clearRect(0, 0, W, H);
			let onscreen = 0;
			for (const p of parts) {
				p.vy += gravity * dt;
				p.vx *= 0.99;
				p.x += p.vx;
				p.y += p.vy;
				p.rot += p.vr;
				if (p.y < H + 30) onscreen++;
				ctx.save();
				ctx.translate(p.x, p.y);
				ctx.rotate(p.rot);
				ctx.fillStyle = p.c;
				ctx.fillRect(-p.w / 2, -p.h / 2, p.w, p.h);
				ctx.restore();
			}
			if (onscreen > 0) {
				raf = requestAnimationFrame(frame);
			} else {
				ctx.clearRect(0, 0, W, H);
			}
		};
		raf = requestAnimationFrame(frame);
	}

	$effect(() => {
		if (trigger > 0) {
			const reduce = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
			if (!reduce) burst();
		}
		return () => cancelAnimationFrame(raf);
	});
</script>

<canvas bind:this={canvas} class="confetti" aria-hidden="true"></canvas>

<style>
	.confetti {
		position: fixed;
		inset: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
		z-index: 65;
	}
</style>
