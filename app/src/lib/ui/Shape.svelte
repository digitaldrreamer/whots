<script lang="ts">
	import type { Shape } from '$lib/game/types.js';

	let {
		shape,
		size = 24,
		color = 'currentColor'
	}: { shape: Shape; size?: number; color?: string } = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 100 100"
	fill={color}
	stroke={color}
	aria-hidden="true"
	class="shape"
>
	{#if shape === 'circle'}
		<circle cx="50" cy="50" r="38" />
	{:else if shape === 'triangle'}
		<path d="M50 12 L88 82 L12 82 Z" stroke-linejoin="round" stroke-width="6" />
	{:else if shape === 'square'}
		<rect x="16" y="16" width="68" height="68" rx="8" />
	{:else if shape === 'cross'}
		<path
			d="M38 12 H62 V38 H88 V62 H62 V88 H38 V62 H12 V38 H38 Z"
			stroke-linejoin="round"
			stroke-width="4"
		/>
	{:else if shape === 'star'}
		<path
			d="M50 8 L61 38 L94 38 L67 58 L77 90 L50 70 L23 90 L33 58 L6 38 L39 38 Z"
			stroke-linejoin="round"
			stroke-width="4"
		/>
	{/if}
</svg>

<style>
	.shape {
		display: block;
	}
</style>
