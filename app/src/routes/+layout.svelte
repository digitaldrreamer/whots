<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import './app.css';
	import { onMount } from 'svelte';
	import { session } from '$lib/stores/session.svelte';
	import { lobby } from '$lib/stores/lobby.svelte';

	let { children } = $props();

	// Restore a session from the httpOnly refresh cookie on load.
	onMount(() => {
		void session.restore();
	});

	// Open the lobby (notify socket + social data) whenever signed in.
	$effect(() => {
		if (session.status === 'authed') void lobby.open();
		else if (session.status === 'anon') lobby.close();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
