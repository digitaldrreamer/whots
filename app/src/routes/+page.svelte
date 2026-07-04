<script lang="ts">
	import { game } from '$lib/ui/game.svelte.js';
	import Menu from '$lib/ui/Menu.svelte';
	import Board from '$lib/ui/Board.svelte';
	import Result from '$lib/ui/Result.svelte';
	import InvitePrompt from '$lib/ui/InvitePrompt.svelte';
	import RoomComposer from '$lib/ui/RoomComposer.svelte';
</script>

<svelte:head>
	<title>Whot! — Nigerian card game</title>
	<meta name="description" content="Play Nigerian Whot against escalating AI opponents." />
</svelte:head>

{#if game.screen === 'menu'}
	<Menu />
{:else if game.screen === 'connecting'}
	<div class="connecting">
		<div class="spinner" aria-hidden="true"></div>
		<p>Dealing you in…</p>
		{#if game.error}
			<p class="err">{game.error}</p>
			<button onclick={() => game.toMenu()}>Back to menu</button>
		{/if}
	</div>
{:else}
	<Board />
	{#if game.screen === 'result'}
		<Result />
	{/if}
{/if}

<!-- Global overlays: game invites + toasts + room composer, on every screen -->
<InvitePrompt />
<RoomComposer />

<style>
	.connecting {
		min-height: 100dvh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		color: rgba(255, 255, 255, 0.75);
	}
	.spinner {
		width: 44px;
		height: 44px;
		border-radius: 50%;
		border: 3px solid rgba(255, 255, 255, 0.15);
		border-top-color: var(--gold, #e8b84b);
		animation: spin 0.8s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	.err {
		color: #ff8fa3;
	}
	.connecting button {
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.15);
		color: #fff;
		padding: 0.5rem 1rem;
		border-radius: 8px;
		cursor: pointer;
	}
</style>
