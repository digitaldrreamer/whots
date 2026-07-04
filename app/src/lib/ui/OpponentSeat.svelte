<script lang="ts">
	import Card from './Card.svelte';
	import { fly } from 'svelte/transition';

	let {
		name,
		handSize,
		active,
		thinking,
		seatIndex,
		isTee = false,
		owed = 0,
		talk = null
	}: {
		name: string;
		handSize: number;
		active: boolean;
		thinking: boolean;
		seatIndex: number;
		isTee?: boolean;
		owed?: number;
		talk?: string | null;
	} = $props();

	const count = $derived(handSize);
	const fan = $derived(Math.min(count, 6));
</script>

<div class="seat" class:active class:tee={isTee} data-seat={seatIndex}>
	{#if talk}
		{#key talk}
			<div class="bubble" transition:fly={{ y: 8, duration: 220 }}>{talk}</div>
		{/key}
	{/if}
	<div class="fan" style:--n={fan}>
		{#each Array.from({ length: fan }, (_, idx) => idx) as i (i)}
			<div class="fslot" style:--i={i}>
				<Card faceDown size="sm" />
			</div>
		{/each}
		{#if count === 0}
			<div class="empty">—</div>
		{/if}
	</div>
	<div class="info">
		<span class="name">
			{#if isTee}👑
			{/if}{name}
		</span>
		<span class="count">{count} card{count === 1 ? '' : 's'}</span>
	</div>
	{#if owed > 0}
		<div class="owed" title="Must draw from market">🛒 picking {owed}</div>
	{/if}
	{#if thinking}
		<div class="thinking">
			<span class="dot"></span><span class="dot"></span><span class="dot"></span>
		</div>
	{/if}
</div>

<style>
	.seat {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.4rem;
		padding: 0.6rem 0.8rem;
		border-radius: 14px;
		border: 1.5px solid transparent;
		transition:
			border-color 0.2s,
			background 0.2s,
			box-shadow 0.2s;
		position: relative;
	}
	.seat.active {
		border-color: var(--gold, #e8b84b);
		background: rgba(232, 184, 75, 0.08);
		box-shadow: 0 0 22px rgba(232, 184, 75, 0.18);
	}
	.seat.tee.active {
		border-color: #ff5470;
		background: rgba(255, 84, 112, 0.1);
		box-shadow: 0 0 26px rgba(255, 84, 112, 0.28);
	}
	.fan {
		position: relative;
		height: 66px;
		width: calc(46px + (var(--n) - 1) * 16px);
		min-width: 46px;
	}
	.fslot {
		position: absolute;
		left: calc(var(--i) * 16px);
		top: 0;
	}
	.empty {
		color: rgba(255, 255, 255, 0.35);
		font-size: 1.4rem;
		line-height: 66px;
	}
	.info {
		display: flex;
		flex-direction: column;
		align-items: center;
		line-height: 1.2;
	}
	.name {
		font-weight: 700;
		color: #fff;
		font-size: 0.95rem;
	}
	.tee .name {
		color: #ff8fa3;
	}
	.count {
		font-size: 0.78rem;
		color: rgba(255, 255, 255, 0.6);
	}
	.bubble {
		position: absolute;
		bottom: calc(100% + 4px);
		left: 50%;
		transform: translateX(-50%);
		z-index: 6;
		max-width: 180px;
		width: max-content;
		background: #fff;
		color: #14201a;
		font-size: 0.78rem;
		font-weight: 600;
		line-height: 1.25;
		padding: 0.4rem 0.6rem;
		border-radius: 12px;
		box-shadow: 0 6px 18px rgba(0, 0, 0, 0.35);
		text-align: center;
		pointer-events: none;
	}
	.bubble::after {
		content: '';
		position: absolute;
		top: 100%;
		left: 50%;
		transform: translateX(-50%);
		border: 6px solid transparent;
		border-top-color: #fff;
	}
	.tee .bubble {
		background: #2a0f16;
		color: #ffd0d8;
	}
	.tee .bubble::after {
		border-top-color: #2a0f16;
	}
	.owed {
		font-size: 0.7rem;
		font-weight: 800;
		color: #1a1205;
		background: var(--gold, #e8b84b);
		padding: 0.1rem 0.5rem;
		border-radius: 999px;
		white-space: nowrap;
		animation: owedPulse 1.1s ease-in-out infinite;
	}
	@keyframes owedPulse {
		50% {
			opacity: 0.6;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.owed {
			animation: none;
		}
	}
	.thinking {
		position: absolute;
		bottom: -14px;
		display: flex;
		gap: 4px;
	}
	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--gold, #e8b84b);
		animation: bounce 1s infinite ease-in-out;
	}
	.dot:nth-child(2) {
		animation-delay: 0.15s;
	}
	.dot:nth-child(3) {
		animation-delay: 0.3s;
	}
	@keyframes bounce {
		0%,
		60%,
		100% {
			transform: translateY(0);
			opacity: 0.5;
		}
		30% {
			transform: translateY(-6px);
			opacity: 1;
		}
	}
</style>
