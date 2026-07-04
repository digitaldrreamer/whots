<script lang="ts">
	import type { Card as CardT } from '$lib/game/types.js';
	import Card from './Card.svelte';

	type Rect = { x: number; y: number; w: number; h: number };
	type Flight = { id: number; card?: CardT; faceDown?: boolean; from: Rect; to: Rect };

	let { flights, ondone }: { flights: Flight[]; ondone: (id: number) => void } = $props();

	// Animate a ghost card along an arc from its start rect to the pile.
	function fly(node: HTMLElement, f: Flight) {
		const dx = f.to.x - f.from.x;
		const dy = f.to.y - f.from.y;
		const lift = Math.min(80, Math.abs(dy) * 0.35 + 24);
		const spin = (Math.random() * 2 - 1) * 10;
		let anim: Animation | null = null;
		try {
			anim = node.animate(
				[
					{ transform: 'translate(0,0) scale(1) rotate(0deg)', opacity: 0.95, offset: 0 },
					{
						transform: `translate(${dx * 0.5}px, ${dy * 0.5 - lift}px) scale(1.08) rotate(${spin}deg)`,
						opacity: 1,
						offset: 0.55
					},
					{ transform: `translate(${dx}px, ${dy}px) scale(1) rotate(0deg)`, opacity: 1, offset: 1 }
				],
				{ duration: 360, easing: 'cubic-bezier(0.45, 0, 0.25, 1)', fill: 'forwards' }
			);
			anim.onfinish = () => ondone(f.id);
			anim.oncancel = () => ondone(f.id);
		} catch {
			ondone(f.id);
		}
		return {
			destroy() {
				anim?.cancel();
			}
		};
	}
</script>

<div class="flight-layer" aria-hidden="true">
	{#each flights as f (f.id)}
		<div
			class="ghost"
			style:left="{f.from.x}px"
			style:top="{f.from.y}px"
			style:width="{f.from.w}px"
			style:height="{f.from.h}px"
			use:fly={f}
		>
			<Card card={f.card} faceDown={f.faceDown} size="lg" />
		</div>
	{/each}
</div>

<style>
	.flight-layer {
		position: fixed;
		inset: 0;
		pointer-events: none;
		z-index: 45;
		overflow: hidden;
	}
	.ghost {
		position: fixed;
		will-change: transform;
	}
	.ghost :global(.card) {
		width: 100% !important;
		height: 100% !important;
	}
	@media (prefers-reduced-motion: reduce) {
		.flight-layer {
			display: none;
		}
	}
</style>
