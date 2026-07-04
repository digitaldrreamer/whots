<script lang="ts">
	import { lobby } from '$lib/stores/lobby.svelte';
	import type { GameMode } from '$lib/api/types';

	let { mode, onclose }: { mode: GameMode; onclose: () => void } = $props();

	let query = $state('');

	async function onSearch() {
		await lobby.search(query);
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />
<div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
	<div class="panel" role="dialog" aria-modal="true" aria-label="Friends">
		<header>
			<h2>Friends</h2>
			<button class="x" onclick={onclose} aria-label="Close">✕</button>
		</header>

		{#if lobby.requests.length > 0}
			<section>
				<span class="label">Requests</span>
				{#each lobby.requests as u (u.id)}
					<div class="person">
						<span class="who">{u.display_name}<small>@{u.username}</small></span>
						<div class="actions">
							<button class="mini go" onclick={() => lobby.acceptRequest(u.username)}>Accept</button>
							<button class="mini" onclick={() => lobby.declineRequest(u.username)}>Decline</button>
						</div>
					</div>
				{/each}
			</section>
		{/if}

		<section>
			<span class="label">Your friends</span>
			{#if lobby.friends.length === 0}
				<p class="empty">No friends yet — search below to add someone.</p>
			{/if}
			{#each lobby.friends as f (f.id)}
				<div class="person">
					<span class="who">{f.display_name}<small>@{f.username}</small></span>
					<div class="actions">
						<button class="mini go" onclick={() => lobby.inviteFriend(f, mode)}>Invite</button>
						<button class="mini danger" onclick={() => lobby.unfriend(f.username)}>Remove</button>
					</div>
				</div>
			{/each}
		</section>

		<section>
			<span class="label">Add a friend</span>
			<div class="search">
				<input placeholder="Search username…" bind:value={query} oninput={onSearch} />
			</div>
			{#each lobby.searchResults as u (u.id)}
				<div class="person">
					<span class="who">{u.display_name}<small>@{u.username}</small></span>
					<button class="mini go" onclick={() => lobby.addFriend(u.username)}>Add</button>
				</div>
			{/each}
		</section>

		{#if lobby.error}<span class="err">{lobby.error}</span>{/if}
	</div>
</div>

<style>
	.scrim {
		position: fixed;
		inset: 0;
		background: rgba(4, 12, 8, 0.72);
		backdrop-filter: blur(3px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 60;
		padding: 1rem;
	}
	.panel {
		background: #14201a;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 16px;
		padding: 1.25rem;
		max-width: 460px;
		width: 100%;
		max-height: 85vh;
		overflow-y: auto;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.5rem;
	}
	h2 {
		margin: 0;
		color: var(--gold, #e8b84b);
		font-size: 1.25rem;
	}
	.x {
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.5);
		font-size: 1rem;
		cursor: pointer;
	}
	section {
		margin-top: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.label {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: rgba(255, 255, 255, 0.45);
		font-weight: 700;
	}
	.empty {
		margin: 0;
		font-size: 0.85rem;
		color: rgba(255, 255, 255, 0.4);
	}
	.person {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.5rem 0.65rem;
		border-radius: 10px;
		background: rgba(255, 255, 255, 0.03);
	}
	.who {
		display: flex;
		flex-direction: column;
		color: #fff;
		font-weight: 600;
		font-size: 0.9rem;
	}
	.who small {
		color: rgba(255, 255, 255, 0.45);
		font-weight: 400;
	}
	.actions {
		display: flex;
		gap: 0.4rem;
	}
	.mini {
		padding: 0.35rem 0.7rem;
		border-radius: 8px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		background: rgba(255, 255, 255, 0.05);
		color: #fff;
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
	}
	.mini.go {
		background: rgba(232, 184, 75, 0.9);
		color: #1a1205;
		border-color: transparent;
	}
	.mini.danger {
		color: #ff8fa3;
	}
	.search input {
		width: 100%;
		padding: 0.6rem 0.8rem;
		border-radius: 10px;
		border: 1.5px solid rgba(255, 255, 255, 0.14);
		background: rgba(255, 255, 255, 0.04);
		color: #fff;
		font-size: 0.9rem;
	}
	.search input:focus {
		outline: none;
		border-color: var(--gold, #e8b84b);
	}
	.err {
		display: block;
		margin-top: 0.75rem;
		font-size: 0.78rem;
		color: #ff8fa3;
	}
</style>
