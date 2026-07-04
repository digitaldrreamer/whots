<script lang="ts">
	import { lobby } from '$lib/stores/lobby.svelte';
	import { session } from '$lib/stores/session.svelte';
	import { DIFFICULTY_META } from './game.svelte.js';
	import type { Difficulty } from '$lib/api/types';

	// Closing the composer leaves the room (host leaving closes it for all).
	function close() {
		lobby.leaveRoom();
	}

	const room = $derived(lobby.room);
	const isHost = $derived(!!room?.am_i_host);
	const total = $derived((room?.members.length ?? 0) + (room?.ais.length ?? 0));
	const full = $derived(room ? total >= room.max_slots : true);
	const emptySlots = $derived(room ? Math.max(0, room.max_slots - total) : 0);
	const invitable = $derived(
		lobby.friends.filter((f) => !room?.members.some((m) => m.user_id === f.id))
	);

	function diffLabel(d: Difficulty): string {
		return DIFFICULTY_META.find((m) => m.id === d)?.label ?? d;
	}

	let showAi = $state(false);
	function addAi(d: Difficulty) {
		lobby.addAi(d);
		showAi = false;
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && room && close()} />

{#if room}
	<div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
		<div class="panel" role="dialog" aria-modal="true" aria-label="Game room">
			<header>
				<h2>Game room</h2>
				<span class="mode">{room.mode === 'stack' ? 'Stack' : 'No-stack'}</span>
				<button class="x" onclick={close} aria-label="Leave room">✕</button>
			</header>

			<section>
				<span class="label">Table — {total}/{room.max_slots}</span>
				<div class="slots">
					{#each room.members as m (m.user_id)}
						<div class="slot human">
							<span class="who">
								{m.user_id === room.host_id ? '👑 ' : ''}{m.username}{m.user_id ===
								session.user?.id
									? ' (you)'
									: ''}
							</span>
						</div>
					{/each}
					{#each room.ais as d, i (i)}
						<div class="slot ai">
							<span class="who">🤖 {diffLabel(d)}</span>
							{#if isHost}
								<button class="mini danger" onclick={() => lobby.removeAi(i)} aria-label="Remove AI"
									>✕</button
								>
							{/if}
						</div>
					{/each}
					{#each Array(emptySlots) as _, i (i)}
						<div class="slot empty">Empty</div>
					{/each}
				</div>
			</section>

			{#if isHost}
				<section>
					<span class="label">Invite friends</span>
					{#if invitable.length === 0}
						<p class="empty">No friends to invite — add some from the Friends menu.</p>
					{/if}
					{#each invitable as f (f.id)}
						<div class="person">
							<span class="who">{f.display_name}<small>@{f.username}</small></span>
							<button class="mini go" disabled={full} onclick={() => lobby.inviteToRoom(f)}
								>Invite</button
							>
						</div>
					{/each}
				</section>

				<section>
					<span class="label">Add AI</span>
					{#if showAi}
						<div class="ai-grid">
							{#each DIFFICULTY_META as d (d.id)}
								<button class="ai-opt" onclick={() => addAi(d.id)}>{d.label}</button>
							{/each}
						</div>
					{:else}
						<button class="add-ai" disabled={full} onclick={() => (showAi = true)}>+ Add an AI</button>
					{/if}
				</section>

				<button class="start" disabled={total < 2} onclick={() => lobby.startRoom()}>
					Start game{total >= 2 ? ` · ${total} players` : ' (need 2+)'}
				</button>
			{:else}
				<p class="waiting">Waiting for the host to start…</p>
			{/if}

			<button class="leave" onclick={close}>Leave room</button>
			{#if lobby.error}<span class="err">{lobby.error}</span>{/if}
		</div>
	</div>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		z-index: 60;
		background: rgba(6, 12, 9, 0.72);
		backdrop-filter: blur(3px);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}
	.panel {
		width: min(440px, 100%);
		max-height: 90dvh;
		overflow-y: auto;
		background: #12201a;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 16px;
		padding: 1.1rem 1.2rem 1.3rem;
		box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}
	header {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}
	header h2 {
		margin: 0;
		font-size: 1.15rem;
		flex: 1;
	}
	.mode {
		font-size: 0.72rem;
		font-weight: 700;
		padding: 0.2rem 0.55rem;
		border-radius: 999px;
		background: rgba(47, 158, 111, 0.18);
		color: #7fe0b3;
	}
	.x {
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.6);
		font-size: 1rem;
		cursor: pointer;
	}
	section {
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
	}
	.label {
		font-size: 0.72rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: rgba(255, 255, 255, 0.45);
		font-weight: 700;
	}
	.slots {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.slot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.55rem 0.75rem;
		border-radius: 10px;
		font-size: 0.9rem;
		color: #fff;
	}
	.slot.human {
		background: rgba(232, 184, 75, 0.12);
	}
	.slot.ai {
		background: rgba(255, 255, 255, 0.05);
	}
	.slot.empty {
		background: rgba(255, 255, 255, 0.02);
		border: 1px dashed rgba(255, 255, 255, 0.12);
		color: rgba(255, 255, 255, 0.3);
		font-style: italic;
	}
	.person {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}
	.who {
		font-weight: 600;
		color: #fff;
		font-size: 0.9rem;
	}
	.who small {
		color: rgba(255, 255, 255, 0.4);
		font-weight: 400;
		margin-left: 0.35rem;
	}
	.empty {
		color: rgba(255, 255, 255, 0.4);
		font-size: 0.85rem;
		margin: 0;
	}
	.mini {
		border: none;
		border-radius: 7px;
		padding: 0.3rem 0.7rem;
		font-size: 0.78rem;
		font-weight: 700;
		cursor: pointer;
	}
	.mini.go {
		background: #2f9e6f;
		color: #fff;
	}
	.mini.danger {
		background: rgba(255, 84, 112, 0.18);
		color: #ff8fa3;
		padding: 0.2rem 0.5rem;
	}
	.mini:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.add-ai {
		align-self: flex-start;
		background: rgba(255, 255, 255, 0.08);
		border: 1px solid rgba(255, 255, 255, 0.14);
		color: #fff;
		padding: 0.5rem 0.9rem;
		border-radius: 9px;
		cursor: pointer;
		font-size: 0.85rem;
	}
	.add-ai:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.ai-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.4rem;
	}
	.ai-opt {
		background: rgba(255, 255, 255, 0.06);
		border: 1px solid rgba(255, 255, 255, 0.12);
		color: #fff;
		padding: 0.5rem;
		border-radius: 8px;
		cursor: pointer;
		font-size: 0.82rem;
		font-weight: 600;
	}
	.ai-opt:hover {
		background: rgba(232, 184, 75, 0.15);
	}
	.start {
		background: linear-gradient(135deg, #2f9e6f, #22795a);
		color: #fff;
		border: none;
		padding: 0.75rem;
		border-radius: 11px;
		font-weight: 800;
		font-size: 0.95rem;
		cursor: pointer;
	}
	.start:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	.waiting {
		text-align: center;
		color: rgba(255, 255, 255, 0.6);
		font-style: italic;
		margin: 0.3rem 0;
	}
	.leave {
		background: none;
		border: 1px solid rgba(255, 84, 112, 0.3);
		color: #ff8fa3;
		padding: 0.5rem;
		border-radius: 9px;
		cursor: pointer;
		font-size: 0.85rem;
	}
	.err {
		color: #ff8fa3;
		font-size: 0.82rem;
	}
</style>
