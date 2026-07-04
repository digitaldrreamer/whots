<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resetPassword } from '$lib/api/auth';

	let password = $state('');
	let busy = $state(false);
	let error = $state<string | null>(null);
	let done = $state(false);

	const token = $derived($page.url.searchParams.get('token') ?? '');

	async function submit() {
		if (password.length < 8) {
			error = 'Password must be at least 8 characters.';
			return;
		}
		busy = true;
		error = null;
		try {
			await resetPassword(token, password);
			done = true;
			setTimeout(() => goto('/'), 1800);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Could not reset your password.';
		} finally {
			busy = false;
		}
	}
</script>

<div class="wrap">
	<div class="card">
		<div class="emoji">🔒</div>
		{#if !token}
			<h2>Invalid link</h2>
			<p class="sub">This reset link is missing its token.</p>
			<button onclick={() => goto('/')}>Back to menu</button>
		{:else if done}
			<h2>Password updated 🎉</h2>
			<p class="sub">You can now sign in. Taking you home…</p>
		{:else}
			<h2>Set a new password</h2>
			<input
				type="password"
				placeholder="New password (min 8)"
				bind:value={password}
				disabled={busy}
				onkeydown={(e) => e.key === 'Enter' && submit()}
			/>
			<button class="go" onclick={submit} disabled={busy}>{busy ? '…' : 'Reset password'}</button>
			{#if error}<p class="err">{error}</p>{/if}
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
		display: flex;
		flex-direction: column;
		gap: 0.7rem;
	}
	.emoji {
		font-size: 3rem;
	}
	h2 {
		margin: 0.2rem 0 0;
		color: #fff;
		font-size: 1.3rem;
	}
	.sub {
		color: rgba(255, 255, 255, 0.65);
		margin: 0;
	}
	input {
		padding: 0.7rem 0.9rem;
		border-radius: 10px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		background: rgba(255, 255, 255, 0.05);
		color: #fff;
		font-size: 0.95rem;
	}
	.go {
		background: linear-gradient(135deg, #e8b84b, #d99a2b);
		color: #1a1205;
		border: none;
		padding: 0.75rem;
		border-radius: 11px;
		font-weight: 800;
		cursor: pointer;
	}
	.err {
		color: #ff8fa3;
		margin: 0;
	}
	button:not(.go) {
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.15);
		color: #fff;
		padding: 0.6rem 1.1rem;
		border-radius: 10px;
		cursor: pointer;
	}
</style>
