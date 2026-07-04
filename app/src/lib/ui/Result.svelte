<script lang="ts">
	import { game } from './game.svelte.js';

	const won = $derived(game.winnerIsMe);
	const winner = $derived(game.winnerName || 'Opponent');
	const teeReward = $derived(game.isTeeGame && won);
</script>

{#if game.teeChallenge}
	<!-- The final boss appears -->
	<div class="scrim boss-scrim">
		<div class="boss-card">
			<div class="crown">👑</div>
			<span class="kicker">A challenger approaches</span>
			<h1>Tee-Noble</h1>
			<p>
				Your streak drew attention. Tee-Noble plays a flawless game — every module, no blind spots,
				no mercy. Beat them and the Tee-Noble Slayer badge is yours forever.
			</p>
			<p class="warn">One shot. Decline or lose, and they vanish.</p>
			<div class="btns">
				<button class="accept" onclick={() => game.acceptTee()}>Accept the challenge</button>
				<button class="decline" onclick={() => game.declineTee()}>Not today</button>
			</div>
		</div>
	</div>
{:else}
	<div class="scrim">
		<div class="result" class:win={won} class:reward={teeReward}>
			{#if teeReward}
				<div class="emoji">🏆</div>
				<span class="kicker">Legendary</span>
				<h1>You beat Tee-Noble</h1>
				<p>The 🏆 <strong>Tee-Noble Slayer</strong> badge is now on your profile — forever. Few ever see this screen.</p>
			{:else if won}
				<div class="emoji">🎉</div>
				<h1>You win!</h1>
				<p>Hand emptied. Clean work.</p>
			{:else}
				<div class="emoji">🃏</div>
				<h1>{winner} wins</h1>
				<p>
					{game.isTeeGame
						? 'Tee-Noble was flawless. They vanish again.'
						: 'Shuffle up and try again.'}
				</p>
			{/if}

			<div class="btns">
				<button class="again" onclick={() => game.playAgain()}>Play again</button>
				<button class="menu" onclick={() => game.toMenu()}>Main menu</button>
			</div>
		</div>
	</div>
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
		z-index: 70;
		padding: 1rem;
		animation: fade 0.3s ease;
	}
	.boss-scrim {
		background: radial-gradient(circle at 50% 40%, rgba(120, 10, 40, 0.6), rgba(4, 6, 8, 0.92));
	}
	@keyframes fade {
		from {
			opacity: 0;
		}
	}
	.result,
	.boss-card {
		background: #14201a;
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 20px;
		padding: 2rem 1.75rem;
		max-width: 420px;
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
	.result.win {
		border-color: rgba(232, 184, 75, 0.4);
	}
	.result.reward {
		border-color: rgba(232, 184, 75, 0.7);
		box-shadow:
			0 24px 70px rgba(0, 0, 0, 0.55),
			0 0 60px rgba(232, 184, 75, 0.25);
	}
	.emoji,
	.crown {
		font-size: 3.5rem;
		line-height: 1;
	}
	.kicker {
		display: block;
		text-transform: uppercase;
		letter-spacing: 0.18em;
		font-size: 0.75rem;
		font-weight: 700;
		color: var(--gold, #e8b84b);
		margin: 0.75rem 0 0.25rem;
	}
	h1 {
		margin: 0.4rem 0 0.6rem;
		font-size: 1.9rem;
		color: #fff;
	}
	p {
		margin: 0 0 0.6rem;
		color: rgba(255, 255, 255, 0.72);
		font-size: 0.95rem;
		line-height: 1.5;
	}
	.warn {
		color: #ff8fa3;
		font-weight: 600;
		font-size: 0.85rem;
	}
	.btns {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		margin-top: 1.4rem;
	}
	button {
		border: none;
		padding: 0.85rem;
		border-radius: 11px;
		font-weight: 800;
		font-size: 1rem;
		cursor: pointer;
		transition:
			transform 0.24s var(--spring),
			filter 0.15s ease;
	}
	button:hover {
		transform: translateY(-2px);
		filter: brightness(1.07);
	}
	.again,
	.accept {
		background: linear-gradient(135deg, #e8b84b, #d99a2b);
		color: #1a1205;
	}
	.boss-card h1 {
		color: #ff8fa3;
		font-size: 2.4rem;
		letter-spacing: 0.02em;
	}
	.accept {
		background: linear-gradient(135deg, #ff5470, #c62a48);
		color: #fff;
	}
	.menu,
	.decline {
		background: rgba(255, 255, 255, 0.08);
		color: rgba(255, 255, 255, 0.85);
	}
</style>
