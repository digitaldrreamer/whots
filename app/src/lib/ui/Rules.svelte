<script lang="ts">
	let { onclose }: { onclose: () => void } = $props();

	const ACTIONS = [
		{ n: '1', name: 'Hold On', desc: 'Next player skips. You play again immediately.' },
		{ n: '2', name: 'Pick Two', desc: 'Next player draws 2 (or counters in stack mode).' },
		{ n: '5', name: 'Pick Three', desc: 'Next player draws 3 (or counters in stack mode).' },
		{ n: '8', name: 'Suspension', desc: 'Next player is skipped entirely.' },
		{ n: '14', name: 'General Market', desc: 'Every other player draws 1 card.' },
		{ n: '20', name: 'Whot', desc: 'Wild — play on anything, then call any shape.' }
	];
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />
<div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
	<div class="sheet" role="dialog" aria-modal="true" aria-label="How to play" tabindex="-1">
		<h2>How to play</h2>
		<p>
			Match the top of the pile by <strong>shape</strong> or <strong>number</strong>. Can't play? Go
			to market and draw. First to empty their hand wins.
		</p>
		<h3>Action cards</h3>
		<ul>
			{#each ACTIONS as a (a.n)}
				<li>
					<span class="badge">{a.n}</span>
					<span><strong>{a.name}</strong> — {a.desc}</span>
				</li>
			{/each}
		</ul>
		<h3>Stack vs No-stack</h3>
		<p class="muted">
			In <strong>stack mode</strong> a Pick Two/Three can be countered by your own 2 or 5, piling
			the penalty onto the next player. In <strong>no-stack</strong> the penalty lands immediately.
		</p>
		<button class="close" onclick={onclose}>Got it</button>
	</div>
</div>

<style>
	.scrim {
		position: fixed;
		inset: 0;
		background: rgba(4, 12, 8, 0.75);
		backdrop-filter: blur(3px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 60;
		padding: 1rem;
	}
	.sheet {
		background: #14201a;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 16px;
		padding: 1.6rem;
		max-width: 460px;
		width: 100%;
		max-height: 88dvh;
		overflow-y: auto;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
	}
	h2 {
		margin: 0 0 0.5rem;
		color: var(--gold, #e8b84b);
	}
	h3 {
		margin: 1.2rem 0 0.5rem;
		font-size: 0.95rem;
		color: #fff;
	}
	p {
		margin: 0;
		color: rgba(255, 255, 255, 0.75);
		font-size: 0.9rem;
		line-height: 1.5;
	}
	.muted {
		color: rgba(255, 255, 255, 0.6);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	li {
		display: flex;
		gap: 0.7rem;
		align-items: flex-start;
		font-size: 0.88rem;
		color: rgba(255, 255, 255, 0.8);
		line-height: 1.4;
	}
	.badge {
		flex: 0 0 auto;
		width: 26px;
		height: 26px;
		border-radius: 7px;
		background: rgba(232, 184, 75, 0.16);
		color: var(--gold, #e8b84b);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 800;
		font-size: 0.82rem;
	}
	strong {
		color: #fff;
	}
	.close {
		margin-top: 1.4rem;
		width: 100%;
		background: var(--gold, #e8b84b);
		color: #1a1205;
		border: none;
		padding: 0.7rem;
		border-radius: 10px;
		font-weight: 700;
		cursor: pointer;
	}
</style>
