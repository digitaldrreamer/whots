<script lang="ts">
	import type { Difficulty, GameMode } from '$lib/api/types';
	import { game, DIFFICULTY_META } from './game.svelte.js';
	import { session, ApiError } from '$lib/stores/session.svelte';
	import Shape from './Shape.svelte';
	import { SHAPE_COLORS } from './theme.js';
	import Rules from './Rules.svelte';

	let mode = $state<GameMode>('stack');
	let difficulty = $state<Difficulty>('chief');
	let opponents = $state(1);
	let showRules = $state(false);

	let username = $state('');
	let authBusy = $state(false);
	let authError = $state<string | null>(null);

	const HERO_SHAPES = ['circle', 'triangle', 'cross', 'square', 'star'] as const;

	async function signIn() {
		const name = username.trim();
		if (name.length < 3) {
			authError = 'Pick a name with at least 3 characters.';
			return;
		}
		authBusy = true;
		authError = null;
		try {
			await session.guest(name);
		} catch (e) {
			authError = e instanceof ApiError ? e.message : 'Could not sign in.';
		} finally {
			authBusy = false;
		}
	}

	function deal() {
		game.start({ mode, difficulty, opponents });
	}
</script>

<div class="menu">
	<div class="hero">
		<div class="hero-shapes">
			{#each HERO_SHAPES as s (s)}
				<Shape shape={s} size={30} color={SHAPE_COLORS[s]} />
			{/each}
		</div>
		<h1>WHOT<span class="s">!</span></h1>
		<p class="tag">The Nigerian card classic. Match, disrupt, empty your hand first.</p>
	</div>

	<div class="panel">
		<div class="field">
			<span class="label">Player</span>
			{#if session.status === 'authed' && session.user}
				<div class="who">
					<span>Playing as <strong>{session.user.display_name}</strong></span>
					<button class="linkbtn" onclick={() => session.logout()}>Sign out</button>
				</div>
			{:else}
				<div class="signin">
					<input
						class="name-input"
						placeholder="Choose a username"
						bind:value={username}
						maxlength="30"
						disabled={authBusy}
						onkeydown={(e) => e.key === 'Enter' && signIn()}
					/>
					<button class="signin-btn" onclick={signIn} disabled={authBusy}>
						{authBusy ? '…' : 'Play as guest'}
					</button>
				</div>
				{#if authError}<span class="err">{authError}</span>{/if}
			{/if}
		</div>

		<div class="field">
			<span class="label">Mode</span>
			<div class="segmented">
				<button class:on={mode === 'stack'} onclick={() => (mode = 'stack')}>
					Stack
					<small>Counter & pile on penalties</small>
				</button>
				<button class:on={mode === 'no_stack'} onclick={() => (mode = 'no_stack')}>
					No-stack
					<small>Penalties resolve at once</small>
				</button>
			</div>
		</div>

		<div class="field">
			<span class="label">Opponents</span>
			<div class="segmented small">
				{#each [1, 2, 3] as n (n)}
					<button class:on={opponents === n} onclick={() => (opponents = n)}>{n}</button>
				{/each}
			</div>
			{#if opponents === 1}
				<span class="hint">One-on-one duels can summon Tee-Noble…</span>
			{/if}
		</div>

		<div class="field">
			<span class="label">Difficulty</span>
			<div class="diffs">
				{#each DIFFICULTY_META as d (d.id)}
					<button class="diff" class:on={difficulty === d.id} onclick={() => (difficulty = d.id)}>
						<span class="diff-name">{d.label}</span>
						<span class="diff-blurb">{d.blurb}</span>
					</button>
				{/each}
			</div>
		</div>

		<button class="play" onclick={deal} disabled={session.status !== 'authed'}>
			Deal me in
		</button>
		{#if game.error}<span class="err center">{game.error}</span>{/if}
		<button class="rules-link" onclick={() => (showRules = true)}>How to play</button>
	</div>
</div>

{#if showRules}
	<Rules onclose={() => (showRules = false)} />
{/if}

<style>
	.menu {
		min-height: 100dvh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem 1rem;
		gap: 2rem;
	}
	.hero {
		text-align: center;
	}
	.hero-shapes {
		display: flex;
		gap: 0.75rem;
		justify-content: center;
		margin-bottom: 0.75rem;
		opacity: 0.95;
	}
	h1 {
		font-size: clamp(3rem, 12vw, 5.5rem);
		margin: 0;
		font-weight: 900;
		letter-spacing: 0.02em;
		color: #fff;
		line-height: 1;
		text-shadow: 0 4px 24px rgba(0, 0, 0, 0.4);
	}
	h1 .s {
		color: var(--gold, #e8b84b);
	}
	.tag {
		margin: 0.75rem auto 0;
		color: rgba(255, 255, 255, 0.7);
		max-width: 26rem;
		font-size: 1rem;
	}

	.panel {
		width: 100%;
		max-width: 520px;
		background: rgba(10, 22, 16, 0.6);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 20px;
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1.4rem;
		backdrop-filter: blur(6px);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
	}
	.label {
		font-size: 0.8rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: rgba(255, 255, 255, 0.5);
		font-weight: 700;
	}
	.hint {
		font-size: 0.78rem;
		color: rgba(255, 132, 155, 0.85);
		font-style: italic;
	}

	.who {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		color: rgba(255, 255, 255, 0.8);
		font-size: 0.92rem;
	}
	.who strong {
		color: var(--gold, #e8b84b);
	}
	.linkbtn {
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.5);
		text-decoration: underline;
		cursor: pointer;
		font-size: 0.8rem;
	}
	.signin {
		display: flex;
		gap: 0.5rem;
	}
	.name-input {
		flex: 1;
		padding: 0.7rem 0.8rem;
		border-radius: 10px;
		border: 1.5px solid rgba(255, 255, 255, 0.14);
		background: rgba(255, 255, 255, 0.04);
		color: #fff;
		font-size: 0.95rem;
	}
	.name-input:focus {
		outline: none;
		border-color: var(--gold, #e8b84b);
	}
	.signin-btn {
		padding: 0.7rem 1rem;
		border-radius: 10px;
		border: none;
		background: rgba(232, 184, 75, 0.9);
		color: #1a1205;
		font-weight: 700;
		cursor: pointer;
		white-space: nowrap;
	}
	.signin-btn:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.err {
		font-size: 0.78rem;
		color: #ff8fa3;
	}
	.err.center {
		text-align: center;
	}
	.play:disabled {
		opacity: 0.45;
		cursor: not-allowed;
		filter: none;
		transform: none;
	}

	.segmented {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}
	.segmented.small {
		grid-template-columns: repeat(3, 1fr);
	}
	.segmented button {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		padding: 0.7rem;
		border-radius: 12px;
		border: 1.5px solid rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
		color: rgba(255, 255, 255, 0.8);
		cursor: pointer;
		font-weight: 700;
		font-size: 0.95rem;
		transition: all 0.15s;
	}
	.segmented small {
		font-weight: 400;
		font-size: 0.72rem;
		color: rgba(255, 255, 255, 0.45);
	}
	.segmented button.on {
		border-color: var(--gold, #e8b84b);
		background: rgba(232, 184, 75, 0.12);
		color: #fff;
	}
	.segmented button:hover:not(.on) {
		border-color: rgba(255, 255, 255, 0.25);
	}

	.diffs {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}
	.diff {
		text-align: left;
		padding: 0.6rem 0.75rem;
		border-radius: 12px;
		border: 1.5px solid rgba(255, 255, 255, 0.1);
		background: rgba(255, 255, 255, 0.03);
		cursor: pointer;
		display: flex;
		flex-direction: column;
		gap: 2px;
		transition: all 0.15s;
	}
	.diff.on {
		border-color: var(--gold, #e8b84b);
		background: rgba(232, 184, 75, 0.1);
	}
	.diff:hover:not(.on) {
		border-color: rgba(255, 255, 255, 0.22);
	}
	.diff-name {
		font-weight: 700;
		color: #fff;
		font-size: 0.9rem;
	}
	.diff-blurb {
		font-size: 0.72rem;
		color: rgba(255, 255, 255, 0.5);
		line-height: 1.25;
	}

	.play {
		margin-top: 0.4rem;
		background: linear-gradient(135deg, #e8b84b, #d99a2b);
		color: #1a1205;
		border: none;
		padding: 0.95rem;
		border-radius: 12px;
		font-size: 1.1rem;
		font-weight: 800;
		cursor: pointer;
		transition:
			transform 0.24s var(--spring),
			filter 0.15s ease;
	}
	.play:hover {
		filter: brightness(1.06);
		transform: translateY(-2px);
	}
	.rules-link {
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.55);
		cursor: pointer;
		text-decoration: underline;
		font-size: 0.85rem;
	}

	@media (max-width: 420px) {
		.diffs {
			grid-template-columns: 1fr;
		}
	}
</style>
