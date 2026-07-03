<script lang="ts">
	import { SHAPES } from '$lib/game/types.js';
	import { SHAPE_COLORS, SHAPE_LABELS } from './theme.js';
	import Shape from './Shape.svelte';

	let { onpick, oncancel }: { onpick: (s: (typeof SHAPES)[number]) => void; oncancel: () => void } =
		$props();
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && oncancel()} />
<div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && oncancel()}>
	<div class="picker" role="dialog" aria-modal="true" aria-label="Call a shape" tabindex="-1">
		<h2>Call a shape</h2>
		<p>Your Whot is wild — choose the shape the next player must match.</p>
		<div class="grid">
			{#each SHAPES as shape (shape)}
				<button class="opt" style:--c={SHAPE_COLORS[shape]} onclick={() => onpick(shape)}>
					<Shape {shape} size={40} color={SHAPE_COLORS[shape]} />
					<span>{SHAPE_LABELS[shape]}</span>
				</button>
			{/each}
		</div>
		<button class="cancel" onclick={oncancel}>Cancel</button>
	</div>
</div>

<style>
	.scrim {
		position: fixed;
		inset: 0;
		background: rgba(4, 12, 8, 0.72);
		backdrop-filter: blur(3px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 50;
		padding: 1rem;
	}
	.picker {
		background: #14201a;
		border: 1px solid rgba(232, 184, 75, 0.3);
		border-radius: 16px;
		padding: 1.5rem;
		max-width: 420px;
		width: 100%;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
		text-align: center;
	}
	h2 {
		margin: 0 0 0.25rem;
		color: var(--gold, #e8b84b);
		font-size: 1.3rem;
	}
	p {
		margin: 0 0 1.1rem;
		color: rgba(255, 255, 255, 0.7);
		font-size: 0.9rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(90px, 1fr));
		gap: 0.6rem;
	}
	.opt {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.4rem;
		padding: 0.9rem 0.5rem;
		background: rgba(255, 255, 255, 0.04);
		border: 1.5px solid transparent;
		border-radius: 12px;
		color: #fff;
		font-weight: 600;
		font-size: 0.85rem;
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			background 0.15s ease,
			transform 0.24s var(--spring);
	}
	.opt:hover {
		border-color: var(--c);
		background: rgba(255, 255, 255, 0.08);
		transform: translateY(-2px);
	}
	.cancel {
		margin-top: 1.1rem;
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.55);
		cursor: pointer;
		font-size: 0.85rem;
		text-decoration: underline;
	}
</style>
