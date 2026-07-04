<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { verifyEmail } from '$lib/api/auth';

	let status = $state<'verifying' | 'done' | 'error'>('verifying');
	let message = $state('');

	$effect(() => {
		const token = $page.url.searchParams.get('token');
		if (!token) {
			status = 'error';
			message = 'This verification link is missing its token.';
			return;
		}
		if (status === 'verifying' && !message) {
			verifyEmail(token)
				.then(() => {
					status = 'done';
					setTimeout(() => goto('/'), 1800);
				})
				.catch((e) => {
					status = 'error';
					message = e instanceof Error ? e.message : 'Could not verify your email.';
				});
		}
	});
</script>

<div class="wrap">
	<div class="card">
		<div class="emoji">✉️</div>
		{#if status === 'verifying'}
			<h2>Verifying your email…</h2>
		{:else if status === 'done'}
			<h2>Email verified 🎉</h2>
			<p class="sub">Taking you home…</p>
		{:else}
			<h2>Hmm.</h2>
			<p class="err">{message}</p>
			<button onclick={() => goto('/')}>Back to menu</button>
		{/if}
	</div>
</div>

<style>
	.wrap {
		min-height: 100dvh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}
	.card {
		background: #14201a;
		border: 1px solid rgba(232, 184, 75, 0.35);
		border-radius: 18px;
		padding: 1.75rem;
		max-width: 400px;
		width: 100%;
		text-align: center;
		box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
	}
	.emoji {
		font-size: 3rem;
	}
	h2 {
		margin: 0.4rem 0 0.2rem;
		color: #fff;
		font-size: 1.3rem;
	}
	.sub {
		color: rgba(255, 255, 255, 0.65);
		margin: 0;
	}
	.err {
		color: #ff8fa3;
	}
	button {
		margin-top: 0.8rem;
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.15);
		color: #fff;
		padding: 0.6rem 1.1rem;
		border-radius: 10px;
		cursor: pointer;
	}
</style>
