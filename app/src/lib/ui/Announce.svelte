<script lang="ts">
	import { scale, fade } from 'svelte/transition';
	import { backOut } from 'svelte/easing';
	import type { AnnounceData } from './game.svelte.js';

	let { data }: { data: AnnounceData | null } = $props();
</script>

{#if data}
	{#key data.id}
		<div class="wrap" aria-live="assertive">
			<div
				class="banner {data.tone}"
				in:scale={{ start: 1.5, opacity: 0, duration: 260, easing: backOut }}
				out:fade={{ duration: 200 }}
			>
				<span class="text">{data.text}</span>
				{#if data.sub}<span class="sub">{data.sub}</span>{/if}
			</div>
		</div>
	{/key}
{/if}

<style>
	.wrap {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: none;
		z-index: 40;
	}
	.banner {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.2rem;
		padding: 0.9rem 2rem;
		border-radius: 16px;
		background: rgba(8, 16, 12, 0.78);
		backdrop-filter: blur(2px);
		box-shadow:
			0 12px 40px rgba(0, 0, 0, 0.5),
			inset 0 0 0 2px var(--edge, rgba(255, 255, 255, 0.25));
		transform-origin: center;
		animation: float 1.3s ease-out forwards;
	}
	.text {
		font-size: clamp(1.8rem, 7vw, 3rem);
		font-weight: 900;
		letter-spacing: 0.04em;
		line-height: 1;
		color: var(--fg, #fff);
		text-shadow: 0 2px 10px rgba(0, 0, 0, 0.6);
	}
	.sub {
		font-size: 0.9rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.14em;
		color: rgba(255, 255, 255, 0.72);
	}

	.good {
		--fg: #ffe08a;
		--edge: rgba(232, 184, 75, 0.6);
	}
	.bad {
		--fg: #ff9b9b;
		--edge: rgba(229, 72, 77, 0.65);
	}
	.wild {
		--fg: #d9b8ff;
		--edge: rgba(139, 92, 246, 0.65);
	}
	.skip {
		--fg: #9ccaff;
		--edge: rgba(59, 130, 246, 0.6);
	}
	.market {
		--fg: #ffcf8f;
		--edge: rgba(224, 144, 42, 0.65);
	}
	.boss {
		--fg: #ff8fa3;
		--edge: rgba(255, 84, 112, 0.75);
	}

	@keyframes float {
		0% {
			transform: translateY(6px);
		}
		18% {
			transform: translateY(-4px);
		}
		100% {
			transform: translateY(-2px);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.banner {
			animation: none;
		}
	}
</style>
