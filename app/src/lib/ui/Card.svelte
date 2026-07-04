<script lang="ts">
	import type { Card } from '$lib/api/types';
	import { SHAPE_COLORS } from './theme.js';
	import Shape from './Shape.svelte';

	let {
		card,
		faceDown = false,
		size = 'md',
		selectable = false,
		disabled = false,
		muted = false,
		selected = false,
		onclick
	}: {
		card?: Card;
		faceDown?: boolean;
		size?: 'sm' | 'md' | 'lg';
		selectable?: boolean;
		disabled?: boolean;
		muted?: boolean;
		selected?: boolean;
		onclick?: () => void;
	} = $props();

	const isWhot = $derived(card !== undefined && card.kind === 'whot');
	const shape = $derived(card !== undefined && card.kind === 'suit' ? card.shape : null);
	const value = $derived(card !== undefined && card.kind === 'suit' ? card.value : null);
	const color = $derived(shape ? SHAPE_COLORS[shape] : '#111');

	const cornerSize = $derived(size === 'lg' ? 22 : size === 'sm' ? 12 : 16);
	const centerSize = $derived(size === 'lg' ? 64 : size === 'sm' ? 30 : 46);
</script>

{#if faceDown || card === undefined}
	<div class="card back {size}" class:selectable aria-hidden="true">
		<div class="back-inner">
			<span class="back-word">WHOT</span>
		</div>
	</div>
{:else}
	{#snippet inner()}
		{#if isWhot}
			<span class="whot-value top">20</span>
			<span class="whot-label">WHOT</span>
			<span class="whot-star">
				<Shape shape="star" size={centerSize} color="url(#whot-grad)" />
			</span>
			<span class="whot-value bottom">20</span>
		{:else if shape}
			<span class="corner top" style:color>
				<span class="num">{value}</span>
				<Shape {shape} size={cornerSize} {color} />
			</span>
			<span class="center">
				<Shape {shape} size={centerSize} {color} />
			</span>
			<span class="corner bottom" style:color>
				<span class="num">{value}</span>
				<Shape {shape} size={cornerSize} {color} />
			</span>
		{/if}
	{/snippet}

	{#if selectable}
		<button
			class="card face {size} selectable"
			class:selected
			class:muted
			class:whot={isWhot}
			style:--card-color={color}
			{disabled}
			type="button"
			aria-label={isWhot ? 'Whot wild card' : `${value} ${shape}`}
			{onclick}
		>
			{@render inner()}
		</button>
	{:else}
		<div
			class="card face {size}"
			class:whot={isWhot}
			style:--card-color={color}
			aria-label={isWhot ? 'Whot wild card' : `${value} ${shape}`}
		>
			{@render inner()}
		</div>
	{/if}
{/if}

<svg width="0" height="0" style="position:absolute" aria-hidden="true">
	<defs>
		<linearGradient id="whot-grad" x1="0" y1="0" x2="1" y2="1">
			<stop offset="0%" stop-color="#f6c453" />
			<stop offset="50%" stop-color="#e5484d" />
			<stop offset="100%" stop-color="#8b5cf6" />
		</linearGradient>
	</defs>
</svg>

<style>
	.card {
		position: relative;
		border-radius: 9px;
		background: var(--card-face, #fbf7ec);
		box-shadow:
			0 1px 2px rgba(0, 0, 0, 0.25),
			0 4px 10px rgba(0, 0, 0, 0.22);
		flex: 0 0 auto;
		user-select: none;
		box-sizing: border-box;
	}

	.card.sm {
		width: 46px;
		height: 64px;
	}
	.card.md {
		width: 68px;
		height: 96px;
	}
	.card.lg {
		width: 104px;
		height: 146px;
	}

	.face {
		border: 1px solid rgba(0, 0, 0, 0.12);
		padding: 0;
		display: block;
		color: inherit;
		font-family: inherit;
		text-align: left;
	}

	button.face {
		cursor: default;
	}

	.selectable {
		cursor: pointer;
		/* Springy lift on the way up, calm settle on the way down. */
		transition:
			transform 0.34s cubic-bezier(0.34, 1.5, 0.5, 1),
			box-shadow 0.28s ease,
			filter 0.3s ease;
	}
	.selectable:hover:not(:disabled) {
		transform: translateY(-14px) scale(1.03);
		box-shadow:
			0 8px 14px rgba(0, 0, 0, 0.32),
			0 18px 30px rgba(0, 0, 0, 0.26);
	}
	.selectable:active:not(:disabled) {
		transform: translateY(-8px) scale(1.01);
		transition-duration: 0.08s;
	}
	.selectable:disabled {
		cursor: default;
	}
	/* Unplayable-on-your-turn: keep the face fully opaque and legible —
	   just drain the colour a touch so playable cards read as the live ones. */
	.muted {
		filter: saturate(0.5) brightness(0.97);
	}
	.selected {
		transform: translateY(-16px) scale(1.03);
		outline: 3px solid var(--gold, #e8b84b);
		box-shadow:
			0 10px 18px rgba(0, 0, 0, 0.35),
			0 0 0 5px rgba(232, 184, 75, 0.25);
	}

	@media (prefers-reduced-motion: reduce) {
		.selectable {
			transition-duration: 0.01ms;
		}
	}

	.corner {
		position: absolute;
		display: flex;
		flex-direction: column;
		align-items: center;
		line-height: 1;
		gap: 1px;
	}
	.corner.top {
		top: 5px;
		left: 5px;
	}
	.corner.bottom {
		bottom: 5px;
		right: 5px;
		transform: rotate(180deg);
	}
	.num {
		font-weight: 800;
		font-size: 0.9em;
	}
	.lg .num {
		font-size: 1.15rem;
	}
	.sm .num {
		font-size: 0.7rem;
	}

	.center {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	/* Whot wild card */
	.whot {
		background: radial-gradient(circle at 50% 35%, #fff 0%, #f4ecff 70%, #eadcff 100%);
	}
	.whot-star {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.whot-label {
		position: absolute;
		top: 8px;
		left: 0;
		right: 0;
		text-align: center;
		font-weight: 900;
		letter-spacing: 0.12em;
		font-size: 0.62rem;
		color: #6b47c9;
	}
	.lg .whot-label {
		font-size: 0.8rem;
	}
	.whot-value {
		position: absolute;
		font-weight: 800;
		font-size: 0.85rem;
		color: #6b47c9;
	}
	.whot-value.top {
		top: 4px;
		left: 6px;
	}
	.whot-value.bottom {
		bottom: 4px;
		right: 6px;
		transform: rotate(180deg);
	}

	/* Card back */
	.back {
		background: linear-gradient(135deg, #1f6f4f 0%, #15543c 100%);
		border: 1px solid rgba(0, 0, 0, 0.3);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 5px;
	}
	.back-inner {
		width: 100%;
		height: 100%;
		border: 2px solid rgba(232, 184, 75, 0.55);
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		background-image: repeating-linear-gradient(
			45deg,
			rgba(255, 255, 255, 0.05) 0,
			rgba(255, 255, 255, 0.05) 4px,
			transparent 4px,
			transparent 8px
		);
	}
	.back-word {
		font-weight: 900;
		letter-spacing: 0.1em;
		color: rgba(232, 184, 75, 0.85);
		font-size: 0.62rem;
		transform: rotate(-45deg);
	}
	.lg .back-word {
		font-size: 0.9rem;
	}
</style>
