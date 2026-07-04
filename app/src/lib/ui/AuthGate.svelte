<script lang="ts">
	import { session } from '$lib/stores/session.svelte';
	import SignIn from './SignIn.svelte';
	import Shape from './Shape.svelte';
	import { SHAPE_COLORS } from './theme.js';

	const SHAPES = ['circle', 'triangle', 'cross', 'square', 'star'] as const;
</script>

{#if session.status === 'anon'}
	<div class="scrim" role="dialog" aria-modal="true" aria-label="Sign in to play">
		<div class="card">
			<div class="shapes">
				{#each SHAPES as s (s)}
					<Shape shape={s} size={22} color={SHAPE_COLORS[s]} />
				{/each}
			</div>
			<h2>Welcome to Whot!</h2>
			<p>Jump in as a guest, or sign in to keep your friends and history.</p>
			<SignIn />
		</div>
	</div>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		z-index: 100;
		background: rgba(4, 12, 8, 0.9);
		backdrop-filter: blur(6px);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}
	.card {
		width: min(400px, 100%);
		background: #12201a;
		border: 1px solid rgba(232, 184, 75, 0.35);
		border-radius: 18px;
		padding: 1.6rem 1.4rem;
		box-shadow: 0 24px 70px rgba(0, 0, 0, 0.6);
		text-align: center;
		animation: pop 0.35s cubic-bezier(0.2, 1.3, 0.5, 1);
	}
	@keyframes pop {
		from {
			transform: scale(0.9);
			opacity: 0;
		}
	}
	.shapes {
		display: flex;
		gap: 0.6rem;
		justify-content: center;
		margin-bottom: 0.5rem;
	}
	h2 {
		margin: 0.2rem 0 0.2rem;
		color: #fff;
		font-size: 1.5rem;
	}
	p {
		color: rgba(255, 255, 255, 0.65);
		margin: 0 0 1.1rem;
		font-size: 0.92rem;
	}
</style>
