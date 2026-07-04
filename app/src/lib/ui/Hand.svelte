<script lang="ts">
	import type { Card as CardT, Shape } from '$lib/api/types';
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
			if (a.kind === 'suit' && b.kind === 'suit') {
				return SHAPE_ORDER[a.shape] - SHAPE_ORDER[b.shape] || a.value - b.value;
			}
			return 0;
		})
	);

	const KEY = 'whot-hand-spread';
	let spread = $state(false);

	$effect(() => {
		const saved = sessionStorage.getItem(KEY);
		if (saved !== null) spread = saved === '1';
	});

	function toggle() {
		spread = !spread;
		sessionStorage.setItem(KEY, spread ? '1' : '0');
	}
</script>

<div class="hand-area">
	<button
		class="toggle"
		onclick={toggle}
		aria-pressed={spread}
		title={spread ? 'Stack cards' : 'Spread cards'}
		aria-label={spread ? 'Stack cards' : 'Spread cards'}
	>
		<svg width="20" height="16" viewBox="0 0 20 16" aria-hidden="true">
			<rect class="b b1" x="1" y="2" width="6" height="12" rx="1.5" />
			<rect class="b b2" x="7" y="2" width="6" height="12" rx="1.5" />
			<rect class="b b3" x="13" y="2" width="6" height="12" rx="1.5" />
		</svg>
		<span>{spread ? 'Stack' : 'Spread'}</span>
	</button>

	<div class="viewport" class:spread>
		<div class="hand" class:spread style:--n={sorted.length}>
			{#each sorted as card, i (i + '-' + card.kind + '-' + (card.kind === 'suit' ? card.shape + card.value : 'w'))}
				{@const playable = canPlay(card)}
				<div class="slot" style:--i={i}>
					<Card
						{card}
						size="lg"
						selectable
						disabled={!enabled || !playable}
						muted={enabled && !playable}
						onclick={() => onplay(card)}
					/>
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.hand-area {
		position: relative;
		width: 100%;
	}

	.toggle {
		position: absolute;
		right: 0.75rem;
		top: -0.35rem;
		z-index: 10;
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.35rem 0.7rem;
		border-radius: 999px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		background: rgba(255, 255, 255, 0.06);
		color: rgba(255, 255, 255, 0.82);
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.2s ease,
			border-color 0.2s ease,
			color 0.2s ease;
	}
	.toggle:hover {
		background: rgba(255, 255, 255, 0.12);
		border-color: rgba(255, 255, 255, 0.25);
		color: #fff;
	}
	.toggle .b {
		fill: currentColor;
		/* Bars sit overlapped (stacked) by default, slide apart when spread. */
		transition: transform 0.4s cubic-bezier(0.34, 1.4, 0.5, 1);
	}
	.toggle[aria-pressed='true'] .b1 {
		transform: translateX(-2px);
	}
	.toggle[aria-pressed='true'] .b3 {
		transform: translateX(2px);
	}
	.toggle[aria-pressed='false'] .b1 {
		transform: translateX(3px);
	}
	.toggle[aria-pressed='false'] .b2 {
		transform: translateX(0);
	}
	.toggle[aria-pressed='false'] .b3 {
		transform: translateX(-3px);
	}

	.viewport {
		display: flex;
		/* Headroom so the hover/select lift is never clipped once this
		   becomes a scroll container in spread mode. */
		padding: 1.75rem 1rem 0.5rem;
		min-height: 182px;
		overflow: visible;
		scroll-behavior: smooth;
	}
	.viewport.spread {
		overflow-x: auto;
		overflow-y: clip;
		overscroll-behavior-x: contain;
	}
	.viewport.spread::-webkit-scrollbar {
		height: 6px;
	}
	.viewport.spread::-webkit-scrollbar-thumb {
		background: rgba(255, 255, 255, 0.18);
		border-radius: 999px;
	}

	.hand {
		display: flex;
		align-items: flex-end;
		/* max-content + auto side margins: centered when it fits, and cleanly
		   scrollable from the left when it doesn't (unlike justify-content). */
		width: max-content;
		margin-inline: auto;
		--overlap: clamp(-64px, calc(-58px + (10 - var(--n)) * 6px), 4px);
		--gap: 12px;
	}

	.slot {
		position: relative;
		/* Ascending cascade: each card sits above the one to its LEFT, so it only
		   ever covers its left neighbour's right edge — every card's top-left
		   number + shape stays visible. Never inverts, whatever the hand size. */
		z-index: var(--i);
		margin-left: var(--overlap);
		/* The headline motion: cards fan apart / draw together in a staggered
		   wave with a gentle overshoot, so it reads as one fluid gesture. */
		transition: margin-left 0.5s cubic-bezier(0.34, 1.28, 0.44, 1);
		transition-delay: calc(var(--i) * 18ms);
	}
	.hand.spread .slot {
		margin-left: var(--gap);
	}
	.slot:first-child {
		margin-left: 0;
	}
	.slot:hover {
		/* Lift the hovered card above the whole cascade. */
		z-index: 100;
	}

	@media (prefers-reduced-motion: reduce) {
		.slot,
		.toggle .b {
			transition-duration: 0.01ms;
			transition-delay: 0ms;
		}
	}
</style>
