<script lang="ts">
	import type { Card as CardT } from '$lib/api/types';
	import { scale } from 'svelte/transition';
	import { backOut } from 'svelte/easing';
	import { game } from './game.svelte.js';
	import { SHAPE_COLORS, SHAPE_LABELS } from './theme.js';
	import { initSound, isMuted, setMuted } from './sound.js';
	import Card from './Card.svelte';
	import Shape from './Shape.svelte';
	import Hand from './Hand.svelte';
	import OpponentSeat from './OpponentSeat.svelte';
	import ShapePicker from './ShapePicker.svelte';
	import Announce from './Announce.svelte';
	import Confetti from './Confetti.svelte';
	import FlightLayer from './FlightLayer.svelte';
	import TeeNobleIntro from './TeeNobleIntro.svelte';
	import NetBadge from './NetBadge.svelte';
	import { net } from '$lib/stores/net.svelte';

	// Poll latency in the background while at the table; stop on unmount.
	$effect(() => {
		net.startPolling();
		return () => net.stopPolling();
	});

	type Rect = { x: number; y: number; w: number; h: number };
	type Flight = { id: number; card?: CardT; faceDown?: boolean; from: Rect; to: Rect };

	const view = $derived(game.view);

	// The face card to show on the discard pile + any called shape.
	const topAsCard = $derived<CardT | null>(
		view ? (view.discard_top.kind === 'whot' ? { kind: 'whot' } : view.discard_top) : null
	);
	const calledShape = $derived(
		view && view.discard_top.kind === 'whot' ? view.discard_top.called_shape : null
	);
	// Re-key the slam animation whenever the top card changes.
	const topKey = $derived(view ? JSON.stringify(view.discard_top) : '');

	// You may always go to market on your turn (voluntary draw), pending pick or not.
	// Gated on a live socket (game.canAct) so a disconnect can't queue plays.
	const drawEnabled = $derived(game.canAct && !game.mustCallShape);

	const status = $derived.by(() => {
		if (!view) return '';
		// A move is in flight to the server — show it so a slow round-trip doesn't
		// feel frozen.
		if (game.pending) return 'Sending…';
		if (game.thinkingName) return `${game.thinkingName} is thinking…`;
		// Not my turn: say whose it is / who we're waiting on (statusLine also
		// calls out a player who still owes a General Market draw).
		if (!game.isMyTurn) return game.statusLine;
		if (game.mustCallShape) return 'The game opened on a Whot — call a shape.';
		if (game.myOwedDraws > 0) {
			return `General Market — go to market and pick ${game.myOwedDraws} card${game.myOwedDraws > 1 ? 's' : ''} to continue.`;
		}
		if (game.selected.length > 0) {
			return `Stacking ${game.selected.length} — tap more of the same number, or press Play.`;
		}
		if (game.pendingPick > 0) {
			return `You're hit with Pick ${game.pendingPick} — counter with a ${game.pendingCard}, or go to market.`;
		}
		if (game.playableCards.length === 0) return 'No playable card — go to market.';
		return 'Your turn — play a card that matches by shape or number.';
	});

	// Cards the market button will make you take: an owed General Market draw
	// first, otherwise the pending penalty.
	const marketOwed = $derived(game.myOwedDraws > 0 ? game.myOwedDraws : game.pendingPick);

	function onplay(card: CardT) {
		// In no-stack a tap plays immediately; in stack mode it builds a selection.
		game.tapCard(card);
	}

	function canPlay(card: CardT): boolean {
		return game.canPlayCard(card);
	}

	let showLog = $state(false);
	let muted = $state(false);

	$effect(() => {
		initSound();
		muted = isMuted();
	});

	function toggleMute() {
		muted = !muted;
		setMuted(muted);
	}

	// Screen shake: bump a local counter whenever the controller signals a hit.
	let shaking = $state(false);
	let lastShake = 0;
	$effect(() => {
		if (game.shakeId !== lastShake) {
			lastShake = game.shakeId;
			if (game.shakeId > 0) {
				shaking = true;
				setTimeout(() => (shaking = false), 420);
			}
		}
	});

	// --- Card flight: a ghost card arcs from a seat/stock to the pile ---
	let flights = $state<Flight[]>([]);
	function rectOf(sel: string, w = 104, h = 146): Rect | null {
		const el = document.querySelector(sel);
		if (!el) return null;
		const r = el.getBoundingClientRect();
		if (r.width === 0 && r.height === 0) return null;
		return { x: r.left + r.width / 2 - w / 2, y: r.top + r.height / 2 - h / 2, w, h };
	}
	function endFlight(id: number) {
		flights = flights.filter((f) => f.id !== id);
	}

	let seenPlay = 0;
	$effect(() => {
		const lp = game.lastPlay;
		if (!lp || lp.id === seenPlay) return;
		seenPlay = lp.id;
		requestAnimationFrame(() => {
			const to = rectOf('.discard .top-wrap');
			const from =
				lp.seat === game.mySeatIndex ? rectOf('.you-area') : rectOf(`[data-seat="${lp.seat}"]`);
			if (from && to) flights = [...flights, { id: lp.id, card: lp.card, from, to }];
		});
	});

	let seenDraw = 0;
	$effect(() => {
		const ld = game.lastDraw;
		if (!ld || ld.id === seenDraw) return;
		seenDraw = ld.id;
		requestAnimationFrame(() => {
			const from = rectOf('.stock .stock-btn');
			const to =
				ld.seat === game.mySeatIndex ? rectOf('.you-area') : rectOf(`[data-seat="${ld.seat}"]`);
			if (!from || !to) return;
			// One ghost card per drawn card, fanned + staggered so a Pick Two/Three
			// (or General Market) reads as the right number of cards.
			const n = Math.max(1, ld.count);
			for (let k = 0; k < n; k++) {
				const jitter = (i: typeof from) => ({ ...i, x: i.x + (k - (n - 1) / 2) * 14 });
				setTimeout(() => {
					flights = [
						...flights,
						{ id: ld.id * 1000 + k, faceDown: true, from: jitter(from), to: jitter(to) }
					];
				}, k * 90);
			}
		});
	});

	// Deal-in cascade when a new game starts.
	let dealing = $state(false);
	let lastDeal = 0;
	$effect(() => {
		if (game.dealSeq !== lastDeal) {
			lastDeal = game.dealSeq;
			if (game.dealSeq > 0) {
				dealing = true;
				setTimeout(() => (dealing = false), 1000);
			}
		}
	});
</script>

{#if game.disconnected}
	<div class="reconnect-banner" role="status">
		<span class="spinner-dot"></span> Reconnecting…
	</div>
{/if}

{#if view}
	<div class="board" class:shaking class:dealing class:offline={game.disconnected}>
		<header class="topbar">
			<button class="ghost" onclick={() => game.toMenu()}>← Leave</button>
			<div class="mode-chip">
				<span class="mode">{view.mode === 'stack' ? 'Stack mode' : 'No-stack'}</span>
				{#if !game.isTeeGame}<span class="diff">{game.tableLabel}</span>{/if}
				{#if game.isTeeGame}<span class="diff boss">Tee-Noble</span>{/if}
			</div>
			<div class="topbar-right">
				<NetBadge />
				<button
					class="ghost icon"
					onclick={toggleMute}
					aria-pressed={muted}
					aria-label={muted ? 'Unmute sound' : 'Mute sound'}
					title={muted ? 'Unmute' : 'Mute'}
				>
					{muted ? '🔇' : '🔊'}
				</button>
				<button class="ghost" onclick={() => (showLog = !showLog)}>
					{showLog ? 'Hide' : 'Log'}
				</button>
			</div>
		</header>

		<section class="opponents">
			{#each game.opponents as opp (opp.index)}
				<OpponentSeat
					name={opp.name}
					handSize={opp.handSize}
					seatIndex={opp.index}
					isTee={opp.isTee}
					active={opp.isCurrent}
					owed={opp.owed}
					talk={game.tableTalk?.seat === opp.index ? game.tableTalk.text : null}
					thinking={game.thinkingName === opp.name && opp.isCurrent}
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
				<span class="pile-label">Market · {view.stock_size}</span>
			</div>

			<div class="pile discard">
				{#if game.pendingPick > 0}
					{#key game.pendingPick}
						<div class="stack-badge" in:scale={{ start: 1.7, duration: 240, easing: backOut }}>
							+{game.pendingPick}
						</div>
					{/key}
				{/if}
				{#if topAsCard}
					<div class="top-wrap">
						{#key topKey}
							<div
								class="slam"
								in:scale={{ start: 1.35, opacity: 0.4, duration: 220, easing: backOut }}
							>
								<Card card={topAsCard} size="lg" />
							</div>
						{/key}
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

		<div class="status" class:you={game.isMyTurn} class:sending={game.pending} aria-live="polite">
			{#if game.pending}<span class="mini-spin"></span>{/if}{status}
		</div>

		<section class="you-area">
			<div class="you-head">
				<span class="you-name">Your hand</span>
				<div class="you-actions">
					{#if game.selected.length > 0}
						<button class="play-btn" disabled={!game.canConfirmSelection} onclick={() => game.playSelected()}>
							Play {game.selected.length > 1 ? `${game.selected.length} cards` : 'card'}
						</button>
						<button class="clear-btn" onclick={() => game.clearSelection()} aria-label="Clear selection">✕</button>
					{/if}
					<button class="market-btn" class:owed={game.myOwedDraws > 0} disabled={!drawEnabled} onclick={() => game.draw()}>
						{marketOwed > 0 ? `Pick ${marketOwed}` : 'Go to market'}
					</button>
				</div>
			</div>
			<Hand
				cards={game.myHand}
				{canPlay}
				canTap={(c) => game.canTapCard(c)}
				enabled={game.canAct}
				isSelected={(c) => game.isSelected(c)}
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

	{#if game.awaitingShape || game.mustCallShape}
		<ShapePicker
			onpick={(s) => game.chooseShape(s)}
			oncancel={() => game.cancelWhot()}
			cancelable={!game.mustCallShape}
			prompt={game.mustCallShape
				? 'The game opened on a Whot — choose the shape the next player must match.'
				: 'Your Whot is wild — choose the shape the next player must match.'}
		/>
	{/if}

	<FlightLayer {flights} ondone={endFlight} />
	<Announce data={game.announce} />
	<Confetti trigger={game.winBurst} />
	<TeeNobleIntro show={game.teeIntro} />
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
		/* Never let the table push the page wide. */
		overflow-x: hidden;
	}
	/* When the socket drops, dim the board. Play/draw are already disabled via
	   game.canAct; "Leave" stays clickable. */
	.board.offline {
		opacity: 0.8;
	}
	.reconnect-banner {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 100;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.5rem;
		background: rgba(198, 42, 72, 0.95);
		color: #fff;
		font-size: 0.85rem;
		font-weight: 700;
		box-shadow: 0 2px 12px rgba(0, 0, 0, 0.4);
	}
	.spinner-dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		border: 2px solid rgba(255, 255, 255, 0.4);
		border-top-color: #fff;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	.board.shaking {
		animation: shake 0.42s cubic-bezier(0.36, 0.07, 0.19, 0.97);
	}
	@keyframes shake {
		10%,
		90% {
			transform: translateX(-2px);
		}
		20%,
		80% {
			transform: translateX(4px);
		}
		30%,
		50%,
		70% {
			transform: translateX(-8px);
		}
		40%,
		60% {
			transform: translateX(8px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.board.shaking {
			animation: none;
		}
	}
	.topbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.topbar-right {
		display: flex;
		gap: 0.4rem;
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
	.ghost.icon {
		padding: 0.4rem 0.55rem;
		font-size: 1rem;
		line-height: 1;
	}
	.ghost:hover {
		background: rgba(255, 255, 255, 0.12);
	}
	.slam {
		display: inline-block;
	}

	/* Deal-in cascade — pierces child scopes to reach hand + opponent cards. */
	.board.dealing :global(.slot),
	.board.dealing :global(.fslot) {
		animation: dealIn 0.5s cubic-bezier(0.2, 1.2, 0.4, 1) both;
		animation-delay: calc(var(--i, 0) * 55ms);
	}
	@keyframes dealIn {
		from {
			transform: translateY(46px) scale(0.6);
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.board.dealing :global(.slot),
		.board.dealing :global(.fslot) {
			animation: none;
		}
	}

	.stack-badge {
		position: absolute;
		top: -14px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 4;
		background: linear-gradient(135deg, #ff5470, #c62a48);
		color: #fff;
		font-weight: 900;
		font-size: 1.1rem;
		padding: 0.15rem 0.7rem;
		border-radius: 999px;
		box-shadow: 0 4px 14px rgba(198, 42, 72, 0.5);
		white-space: nowrap;
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
	.pile.discard {
		position: relative;
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
	.status.sending {
		color: rgba(255, 255, 255, 0.85);
		font-weight: 600;
	}
	.mini-spin {
		display: inline-block;
		width: 12px;
		height: 12px;
		margin-right: 0.4rem;
		vertical-align: -1px;
		border-radius: 50%;
		border: 2px solid rgba(255, 255, 255, 0.25);
		border-top-color: var(--gold, #e8b84b);
		animation: spin 0.6s linear infinite;
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
	.you-actions {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.play-btn {
		background: linear-gradient(135deg, #2f9e6f, #22795a);
		color: #fff;
		border: none;
		padding: 0.5rem 1rem;
		border-radius: 9px;
		font-weight: 800;
		cursor: pointer;
		font-size: 0.85rem;
		transition: filter 0.15s ease, transform 0.24s var(--spring);
	}
	.play-btn:not(:disabled):hover {
		filter: brightness(1.08);
		transform: translateY(-1px);
	}
	.play-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.clear-btn {
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.14);
		color: rgba(255, 255, 255, 0.7);
		width: 30px;
		height: 30px;
		border-radius: 8px;
		cursor: pointer;
		font-size: 0.8rem;
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
	.market-btn.owed:not(:disabled) {
		animation: owedNudge 1.1s ease-in-out infinite;
	}
	@keyframes owedNudge {
		50% {
			filter: brightness(1.18);
			transform: translateY(-2px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.market-btn.owed:not(:disabled) {
			animation: none;
		}
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
