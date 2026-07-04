<script lang="ts">
	import { game } from '$lib/ui/game.svelte.js';
	import Menu from '$lib/ui/Menu.svelte';
	import Board from '$lib/ui/Board.svelte';
	import Result from '$lib/ui/Result.svelte';
</script>

<svelte:head>
	<title>Whot! — Nigerian card game</title>
	<meta name="description" content="Play Nigerian Whot against escalating AI opponents." />
</svelte:head>

{#if game.screen === 'menu'}
	<Menu />
{:else}
	<Board />
	{#if game.screen === 'result'}
		<Result />
	{/if}
{/if}
