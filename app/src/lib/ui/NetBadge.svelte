<script lang="ts">
	import { net } from '$lib/stores/net.svelte';

	// `detail` = also show download/upload Mbps (menu). Tapping re-tests.
	let { detail = false }: { detail?: boolean } = $props();

	const level = $derived(net.level ?? 'idle');
	const title = $derived(
		net.latency != null
			? `${net.latency} ms${net.jitter != null ? ` · ±${net.jitter} ms jitter` : ''}${net.down != null ? ` · ↓${net.down} ↑${net.up ?? '–'} Mbps` : ''} — tap to re-test`
			: 'Tap to test your connection'
	);
</script>

<button
	class="net {level}"
	class:busy={net.running}
	onclick={() => net.measure(detail)}
	{title}
	aria-label="Connection quality"
>
	<span class="dot"></span>
	{#if net.latency != null}
		<span class="ms">{net.latency} ms</span>
		{#if detail && net.down != null}
			<span class="sp">↓{net.down} ↑{net.up ?? '–'} Mbps</span>
		{/if}
	{:else}
		<span class="ms">{net.running ? 'testing…' : 'test'}</span>
	{/if}
</button>

<style>
	.net {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 999px;
		padding: 0.3rem 0.7rem;
		color: rgba(255, 255, 255, 0.8);
		font-size: 0.74rem;
		font-weight: 600;
		cursor: pointer;
		line-height: 1;
	}
	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #888;
		flex: none;
	}
	.net.good .dot {
		background: #3ddc84;
		box-shadow: 0 0 7px #3ddc84;
	}
	.net.ok .dot {
		background: #e8b84b;
		box-shadow: 0 0 7px #e8b84b;
	}
	.net.poor .dot {
		background: #ff8a3d;
		box-shadow: 0 0 7px #ff8a3d;
	}
	.net.busy .dot {
		animation: pulse 0.9s ease-in-out infinite;
	}
	@keyframes pulse {
		50% {
			opacity: 0.35;
		}
	}
	.sp {
		color: rgba(255, 255, 255, 0.5);
	}
	@media (prefers-reduced-motion: reduce) {
		.net.busy .dot {
			animation: none;
		}
	}
</style>
