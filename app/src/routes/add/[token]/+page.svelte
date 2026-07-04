<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { session } from '$lib/stores/session.svelte';
	import { lobby } from '$lib/stores/lobby.svelte';
	import { redeemInvite } from '$lib/api/invites';
	import SignIn from '$lib/ui/SignIn.svelte';

	let status = $state<'idle' | 'redeeming' | 'done' | 'error'>('idle');
	let message = $state('');

	async function tryRedeem(token: string) {
		if (status !== 'idle') return;
		status = 'redeeming';
		try {
			const { display_name } = await redeemInvite(token);
			message = `You're now friends with ${display_name}! 🎉`;
			status = 'done';
			await lobby.refresh();
			setTimeout(() => goto('/'), 1800);
		} catch (e) {
			message = e instanceof Error ? e.message : 'Could not redeem this invite.';
			status = 'error';
		}
	}

	// Redeem as soon as we're signed in.
	$effect(() => {
		const token = $page.params.token;
		if (token && session.status === 'authed' && status === 'idle') void tryRedeem(token);
	});
</script>

<div class="wrap">
	<div class="card">
		<div class="emoji">🤝</div>
		{#if session.status === 'loading'}
			<p>Loading…</p>
		{:else if session.status !== 'authed'}
			<h2>You've been invited</h2>
			<p class="sub">Sign in (or play as a guest) to become friends.</p>
			<SignIn />
		{:else if status === 'redeeming'}
			<h2>Adding your friend…</h2>
		{:else if status === 'done'}
			<h2>{message}</h2>
			<p class="sub">Taking you to the game…</p>
		{:else if status === 'error'}
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
		margin: 0 0 1rem;
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
