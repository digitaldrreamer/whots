<script lang="ts">
	import { fade } from 'svelte/transition';

	let { show }: { show: boolean } = $props();
</script>

{#if show}
	<div class="scrim" transition:fade={{ duration: 260 }} aria-hidden="true">
		<div class="vignette"></div>
		<div class="content">
			<span class="crown">👑</span>
			<span class="kicker">A challenger approaches</span>
			<h1 class="name">TEE&#8209;NOBLE</h1>
			<span class="tag">no mercy</span>
		</div>
	</div>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		z-index: 60;
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: none;
		background: radial-gradient(circle at 50% 45%, rgba(90, 6, 26, 0.55), rgba(3, 5, 8, 0.94));
	}
	.vignette {
		position: absolute;
		inset: 0;
		box-shadow: inset 0 0 220px 60px rgba(0, 0, 0, 0.85);
		animation: pulse 1.8s ease-in-out infinite;
	}
	.content {
		position: relative;
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.35rem;
	}
	.crown {
		font-size: 3.4rem;
		animation: drop 0.5s cubic-bezier(0.2, 1.3, 0.4, 1) both;
	}
	.kicker {
		text-transform: uppercase;
		letter-spacing: 0.24em;
		font-size: 0.8rem;
		font-weight: 700;
		color: rgba(255, 143, 163, 0.9);
		animation: fadeUp 0.5s ease 0.15s both;
	}
	.name {
		margin: 0;
		font-size: clamp(2.6rem, 12vw, 5.5rem);
		font-weight: 900;
		letter-spacing: 0.04em;
		color: #ff5470;
		text-shadow:
			0 0 30px rgba(255, 84, 112, 0.6),
			0 4px 16px rgba(0, 0, 0, 0.7);
		animation: slam 0.5s cubic-bezier(0.3, 1.4, 0.5, 1) 0.1s both;
	}
	.tag {
		text-transform: uppercase;
		letter-spacing: 0.3em;
		font-size: 0.95rem;
		font-weight: 800;
		color: rgba(255, 255, 255, 0.85);
		animation: fadeUp 0.5s ease 0.3s both;
	}

	@keyframes slam {
		from {
			transform: scale(2.2);
			opacity: 0;
			filter: blur(6px);
		}
	}
	@keyframes drop {
		from {
			transform: translateY(-30px) scale(0.6);
			opacity: 0;
		}
	}
	@keyframes fadeUp {
		from {
			transform: translateY(10px);
			opacity: 0;
		}
	}
	@keyframes pulse {
		50% {
			box-shadow: inset 0 0 260px 80px rgba(0, 0, 0, 0.92);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.crown,
		.kicker,
		.name,
		.tag,
		.vignette {
			animation: none;
		}
	}
</style>
