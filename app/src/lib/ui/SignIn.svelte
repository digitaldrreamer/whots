<script lang="ts">
	import { session, ApiError } from '$lib/stores/session.svelte';

	type Tab = 'guest' | 'login' | 'register';
	let tab = $state<Tab>('guest');
	let busy = $state(false);
	let error = $state<string | null>(null);

	let username = $state('');
	let email = $state('');
	let password = $state('');
	let identifier = $state('');

	async function run(fn: () => Promise<void>) {
		busy = true;
		error = null;
		try {
			await fn();
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Something went wrong.';
		} finally {
			busy = false;
		}
	}

	const guest = () => run(() => session.guest(username.trim()));
	const login = () => run(() => session.login(identifier.trim(), password));
	const register = () => run(() => session.register(username.trim(), email.trim(), password));
</script>

<div class="signin">
	<div class="tabs">
		<button class:on={tab === 'guest'} onclick={() => (tab = 'guest')}>Guest</button>
		<button class:on={tab === 'login'} onclick={() => (tab = 'login')}>Log in</button>
		<button class:on={tab === 'register'} onclick={() => (tab = 'register')}>Sign up</button>
	</div>

	{#if tab === 'guest'}
		<div class="row">
			<input
				placeholder="Choose a username"
				bind:value={username}
				maxlength="30"
				disabled={busy}
				onkeydown={(e) => e.key === 'Enter' && guest()}
			/>
			<button class="go" onclick={guest} disabled={busy}>{busy ? '…' : 'Play as guest'}</button>
		</div>
		<p class="note">No account needed — jump straight in. You can register later to keep friends.</p>
	{:else if tab === 'login'}
		<input placeholder="Username or email" bind:value={identifier} disabled={busy} />
		<input type="password" placeholder="Password" bind:value={password} disabled={busy}
			onkeydown={(e) => e.key === 'Enter' && login()} />
		<button class="go" onclick={login} disabled={busy}>{busy ? '…' : 'Log in'}</button>
	{:else}
		<input placeholder="Username" bind:value={username} maxlength="30" disabled={busy} />
		<input type="email" placeholder="Email" bind:value={email} disabled={busy} />
		<input type="password" placeholder="Password (min 8)" bind:value={password} disabled={busy}
			onkeydown={(e) => e.key === 'Enter' && register()} />
		<button class="go" onclick={register} disabled={busy}>{busy ? '…' : 'Create account'}</button>
	{/if}

	{#if error}<span class="err">{error}</span>{/if}
</div>

<style>
	.signin {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
	}
	.tabs {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 0.35rem;
		margin-bottom: 0.25rem;
	}
	.tabs button {
		padding: 0.4rem;
		border-radius: 8px;
		border: 1px solid rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
		color: rgba(255, 255, 255, 0.7);
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.tabs button.on {
		border-color: var(--gold, #e8b84b);
		background: rgba(232, 184, 75, 0.12);
		color: #fff;
	}
	.row {
		display: flex;
		gap: 0.5rem;
	}
	.row input {
		flex: 1;
	}
	input {
		padding: 0.7rem 0.8rem;
		border-radius: 10px;
		border: 1.5px solid rgba(255, 255, 255, 0.14);
		background: rgba(255, 255, 255, 0.04);
		color: #fff;
		font-size: 0.95rem;
	}
	input:focus {
		outline: none;
		border-color: var(--gold, #e8b84b);
	}
	.go {
		padding: 0.7rem 1rem;
		border-radius: 10px;
		border: none;
		background: rgba(232, 184, 75, 0.9);
		color: #1a1205;
		font-weight: 700;
		cursor: pointer;
		white-space: nowrap;
	}
	.go:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.note {
		margin: 0;
		font-size: 0.76rem;
		color: rgba(255, 255, 255, 0.45);
	}
	.err {
		font-size: 0.78rem;
		color: #ff8fa3;
	}
</style>
