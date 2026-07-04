<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import './app.css';
	import { onMount } from 'svelte';
	import { session } from '$lib/stores/session.svelte';

	let { children } = $props();

	// Restore a session from the httpOnly refresh cookie on load.
	onMount(() => {
		void session.restore();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
