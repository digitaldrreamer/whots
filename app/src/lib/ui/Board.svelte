<script lang="ts">
	import type { Card as CardT } from '$lib/game/types.js';
	import { game } from './game.svelte.js';
	import { SHAPE_COLORS, SHAPE_LABELS } from './theme.js';
	import Card from './Card.svelte';
	import Shape from './Shape.svelte';
	import Hand from './Hand.svelte';
	import OpponentSeat from './OpponentSeat.svelte';
	import ShapePicker from './ShapePicker.svelte';

	const gs = $derived(game.state);
	const opponents = $derived(gs ? gs.players.slice(1) : []);

	// The face card to show on the discard pile + any called shape.
	const topAsCard = $derived<CardT | null>(
		gs ? (gs.topCard.kind === 'whot' ? { kind: 'whot', value: 20 } : gs.topCard) : null
	);
	const calledShape = $derived(gs && gs.topCard.kind === 'whot' ? gs.topCard.calledShape : null);

	const drawEnabled = $derived(
		game.isHumanTurn && (game.pendingPick > 0 || game.validHumanCards.length === 0)
	);

	const status = $derived.by(() => {
		if (!gs) return '';
		if (game.busy && game.thinkingName) return `${game.thinkingName} is thinking…`;
		if (!game.isHumanTurn) return '';
		if (game.pendingPick > 0) {
			return `You're hit with Pick ${game.pendingPick} — counter with a 2 or 5, or go to market.`;
		}
		if (game.validHumanCards.length === 0) return 'No playable card — go to market.';
		return 'Your turn — play a card that matches by shape or number.';
	});

	function onplay(card: CardT) {
		if (card.kind === 'whot') game.beginWhot();
		else game.playSuit(card);
	}

	let showLog = $state(false);
</script>

{#if gs}
	<div class="board">
		<header class="topbar">
			<button class="ghost" onclick={() => game.toMenu()}>← Leave</button>
			<div class="mode-chip">
				<span class="mode">{gs.mode === 'stack' ? 'Stack mode' : 'No-stack'}</span>
				{#if !game.isTeeGame}<span class="diff">{game.difficultyLabel}</span>{/if}
				{#if game.isTeeGame}<span class="diff boss">Tee-Noble</span>{/if}
			</div>
			<button class="ghost" onclick={() => (showLog = !showLog)}>
				{showLog ? 'Hide' : 'Log'}
			</button>
		</header>

		<section class="opponents">
			{#each opponents as opp, i (opp.id)}
				<OpponentSeat
					player={opp}
					active={gs.currentPlayerIndex === i + 1 && gs.phase === 'playing'}
					thinking={game.busy && game.thinkingName === opp.name}
				/>
			{/each}
		</section>

		<section class="table">
			<div class="pile stock">
				<button
					class="stock-btn"
					disabled={!drawEnabled}
					onclick={() => game.draw()}
					aria-label="Draw from market"
				>
					<Card faceDown size="lg" />
				</button>
				<span class="pile-label">Market · {gs.stockPile.length}</span>
			</div>

			<div class="pile discard">
				{#if topAsCard}
					<div class="top-wrap">
						<Card card={topAsCard} size="lg" />
						{#if calledShape}
							<div class="called" style:--c={SHAPE_COLORS[calledShape]}>
								<Shape shape={calledShape} size={16} color={SHAPE_COLORS[calledShape]} />
								<span>{SHAPE_LABELS[calledShape]}</span>
							</div>
						{/if}
					</div>
				{/if}
				<span class="pile-label">Pile</span>
			</div>
		</section>

		<div class="status" class:you={game.isHumanTurn} aria-live="polite">{status}</div>

		<section class="you-area">
			<div class="you-head">
				<span class="you-name">Your hand</span>
				<button class="market-btn" disabled={!drawEnabled} onclick={() => game.draw()}>
					{game.pendingPick > 0 ? `Pick ${game.pendingPick}` : 'Go to market'}
				</button>
			</div>
			<Hand
				cards={game.human?.hand ?? []}
				canPlay={game.canPlayCard}
				enabled={game.isHumanTurn}
				{onplay}
			/>
		</section>

		{#if showLog}
			<aside class="log">
				<h3>Play-by-play</h3>
				<ul>
					{#each [...game.log].reverse() as entry (entry.id)}
						<li class={entry.who}>{entry.text}</li>
					{/each}
				</ul>
			</aside>
		{/if}
	</div>

	{#if game.awaitingShape}
		<ShapePicker onpick={(s) => game.chooseShape(s)} oncancel={() => game.cancelWhot()} />
	{/if}
{/if}

<style>
	.board {
		position: relative;
		display: flex;
		flex-direction: column;
		min-height: 100dvh;
		max-width: 1000px;
		margin: 0 auto;
		padding: 0.75rem 1rem 1rem;
	}
	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.ghost {
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		color: rgba(255, 255, 255, 0.85);
		padding: 0.4rem 0.8rem;
		border-radius: 8px;
		cursor: pointer;
		font-size: 0.85rem;
	}
	.ghost:hover {
		background: rgba(255, 255, 255, 0.12);
	}
	.mode-chip {
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	.mode,
	.diff {
		font-size: 0.75rem;
		padding: 0.25rem 0.6rem;
		border-radius: 999px;
		font-weight: 600;
	}
	.mode {
		background: rgba(47, 158, 111, 0.18);
		color: #7fe0b3;
	}
	.diff {
		background: rgba(232, 184, 75, 0.16);
		color: var(--gold, #e8b84b);
	}
	.diff.boss {
		background: rgba(255, 84, 112, 0.16);
		color: #ff8fa3;
	}

	.opponents {
		display: flex;
		justify-content: center;
		flex-wrap: wrap;
		gap: 1rem;
		padding: 1.2rem 0 0.5rem;
	}

	.table {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: clamp(1.5rem, 8vw, 4rem);
		flex: 1;
		padding: 1rem 0;
	}
	.pile {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
	}
	.pile-label {
		font-size: 0.78rem;
		color: rgba(255, 255, 255, 0.55);
		font-weight: 600;
	}
	.stock-btn {
		background: none;
		border: none;
		padding: 0;
		border-radius: 9px;
		cursor: pointer;
		transition: transform 0.28s var(--spring);
	}
	.stock-btn:not(:disabled):hover {
		transform: translateY(-6px);
	}
	.stock-btn:disabled {
		cursor: default;
		opacity: 0.85;
	}
	.top-wrap {
		position: relative;
	}
	.called {
		position: absolute;
		bottom: -10px;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 4px;
		background: #14201a;
		border: 1.5px solid var(--c);
		color: #fff;
		padding: 2px 8px;
		border-radius: 999px;
		font-size: 0.72rem;
		font-weight: 700;
		white-space: nowrap;
	}

	.status {
		text-align: center;
		min-height: 1.5rem;
		font-size: 0.92rem;
		color: rgba(255, 255, 255, 0.7);
		padding: 0.3rem 1rem;
		transition: color 0.2s;
	}
	.status.you {
		color: var(--gold, #e8b84b);
		font-weight: 600;
	}

	.you-area {
		margin-top: auto;
	}
	.you-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 0.5rem;
	}
	.you-name {
		font-weight: 700;
		color: rgba(255, 255, 255, 0.85);
		font-size: 0.9rem;
	}
	.market-btn {
		background: var(--gold, #e8b84b);
		color: #1a1205;
		border: none;
		padding: 0.5rem 1rem;
		border-radius: 9px;
		font-weight: 700;
		cursor: pointer;
		font-size: 0.85rem;
		transition:
			transform 0.24s var(--spring),
			filter 0.15s ease;
	}
	.market-btn:not(:disabled):hover {
		filter: brightness(1.08);
		transform: translateY(-1px);
	}
	.market-btn:disabled {
		background: rgba(255, 255, 255, 0.08);
		color: rgba(255, 255, 255, 0.35);
		cursor: not-allowed;
	}

	.log {
		position: fixed;
		right: 0;
		top: 0;
		bottom: 0;
		width: min(320px, 85vw);
		background: rgba(10, 20, 15, 0.97);
		border-left: 1px solid rgba(255, 255, 255, 0.1);
		padding: 1rem;
		overflow-y: auto;
		z-index: 20;
		box-shadow: -10px 0 40px rgba(0, 0, 0, 0.4);
	}
	.log h3 {
		margin: 0 0 0.75rem;
		color: var(--gold, #e8b84b);
		font-size: 0.95rem;
	}
	.log ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.log li {
		font-size: 0.82rem;
		line-height: 1.35;
		padding-left: 0.6rem;
		border-left: 2px solid rgba(255, 255, 255, 0.15);
		color: rgba(255, 255, 255, 0.75);
	}
	.log li.you {
		border-left-color: var(--gold, #e8b84b);
		color: #fff;
	}
	.log li.them {
		border-left-color: #ff8fa3;
	}
	.log li.system {
		border-left-color: #7fe0b3;
		font-style: italic;
		color: rgba(255, 255, 255, 0.6);
	}

	@media (max-width: 560px) {
		.table {
			gap: 1.5rem;
		}
	}
</style>
