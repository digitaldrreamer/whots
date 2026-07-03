<script lang="ts">
	import type { Card as CardT, Shape } from '$lib/game/types.js';
	import { isSuitCard } from '$lib/game/guards.js';
	import Card from './Card.svelte';

	let {
		cards,
		canPlay,
		enabled,
		onplay
	}: {
		cards: CardT[];
		canPlay: (card: CardT) => boolean;
		enabled: boolean;
		onplay: (card: CardT) => void;
	} = $props();

	const SHAPE_ORDER: Record<Shape, number> = {
		circle: 0,
		triangle: 1,
		cross: 2,
		square: 3,
		star: 4
	};

	// Stable, readable ordering: group by shape then value, Whot cards last.
	const sorted = $derived(
		[...cards].sort((a, b) => {
			if (a.kind === 'whot' && b.kind === 'whot') return 0;
			if (a.kind === 'whot') return 1;
			if (b.kind === 'whot') return -1;
			if (isSuitCard(a) && isSuitCard(b)) {
				return SHAPE_ORDER[a.shape] - SHAPE_ORDER[b.shape] || a.value - b.value;
			}
			return 0;
		})
	);
</script>

<div class="hand" style:--n={sorted.length}>
	{#each sorted as card, i (i + '-' + card.kind + '-' + (isSuitCard(card) ? card.shape + card.value : 'w'))}
		{@const playable = canPlay(card)}
		<div class="slot" style:--i={i}>
			<Card
				{card}
				size="lg"
				selectable
				disabled={!enabled || !playable}
				onclick={() => onplay(card)}
			/>
		</div>
	{/each}
</div>

<style>
	.hand {
		display: flex;
		justify-content: center;
		align-items: flex-end;
		padding: 1.4rem 1rem 0.4rem;
		min-height: 150px;
		/* Overlap cards when the hand grows large */
		--overlap: clamp(-58px, calc(-58px + (10 - var(--n)) * 6px), 6px);
	}
	.slot {
		margin-left: var(--overlap);
		transition: margin 0.2s ease;
	}
	.slot:first-child {
		margin-left: 0;
	}
	.slot:hover {
		z-index: 5;
	}
</style>
