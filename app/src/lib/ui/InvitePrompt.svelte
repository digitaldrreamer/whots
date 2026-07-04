<script lang="ts">
	import { lobby } from '$lib/stores/lobby.svelte';
</script>

{#if lobby.pendingInvite}
	<div class="scrim">
		<div class="card">
			<div class="emoji">🎴</div>
			<span class="kicker">Game invite</span>
			<h2>{lobby.pendingInvite.from} wants to play</h2>
			<div class="btns">
				<button class="accept" onclick={() => lobby.acceptInvite()}>Join game</button>
				<button class="decline" onclick={() => lobby.declineInvite()}>Decline</button>
			</div>
		</div>
	</div>
{/if}

{#if lobby.pendingRoomInvite && !lobby.room}
	<div class="scrim">
		<div class="card">
			<div class="emoji">👥</div>
			<span class="kicker">Room invite</span>
			<h2>{lobby.pendingRoomInvite.from} invited you to a room</h2>
			<div class="btns">
				<button
					class="accept"
					onclick={() => {
						const r = lobby.pendingRoomInvite;
						if (r) lobby.joinRoom(r.roomId);
					}}>Join room</button
				>
				<button class="decline" onclick={() => lobby.declineRoomInvite()}>Decline</button>
			</div>
		</div>
	</div>
{/if}

{#if lobby.toast}
	<div class="toast">{lobby.toast}</div>
{/if}

<style>
	.scrim {
		position: fixed;
		inset: 0;
		background: rgba(4, 12, 8, 0.8);
		backdrop-filter: blur(4px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 80;
		padding: 1rem;
	}
	.card {
		background: #14201a;
		border: 1px solid rgba(232, 184, 75, 0.4);
		border-radius: 18px;
		padding: 1.75rem;
		max-width: 380px;
		width: 100%;
		text-align: center;
		box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
		animation: pop 0.35s cubic-bezier(0.2, 1.3, 0.5, 1);
	}
	@keyframes pop {
		from {
			transform: scale(0.85);
			opacity: 0;
		}
	}
	.emoji {
		font-size: 3rem;
	}
	.kicker {
		display: block;
		text-transform: uppercase;
		letter-spacing: 0.16em;
		font-size: 0.72rem;
		font-weight: 700;
		color: var(--gold, #e8b84b);
		margin: 0.5rem 0 0.2rem;
	}
	h2 {
		margin: 0.2rem 0 0;
		font-size: 1.35rem;
		color: #fff;
	}
	.btns {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		margin-top: 1.3rem;
	}
	button {
		border: none;
		padding: 0.8rem;
		border-radius: 11px;
		font-weight: 800;
		font-size: 1rem;
		cursor: pointer;
	}
	.accept {
		background: linear-gradient(135deg, #e8b84b, #d99a2b);
		color: #1a1205;
	}
	.decline {
		background: rgba(255, 255, 255, 0.08);
		color: rgba(255, 255, 255, 0.85);
	}
	.toast {
		position: fixed;
		bottom: 1.5rem;
		left: 50%;
		transform: translateX(-50%);
		z-index: 90;
		background: rgba(20, 32, 26, 0.97);
		border: 1px solid rgba(255, 255, 255, 0.12);
		color: #fff;
		padding: 0.7rem 1.2rem;
		border-radius: 999px;
		font-size: 0.88rem;
		box-shadow: 0 8px 30px rgba(0, 0, 0, 0.4);
		animation: rise 0.3s ease;
	}
	@keyframes rise {
		from {
			opacity: 0;
			transform: translate(-50%, 8px);
		}
	}
</style>
