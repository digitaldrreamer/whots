<script lang="ts">
	import { onMount } from 'svelte';
	import type { Difficulty, GameMode, GameSummary } from '$lib/api/types';
	import { timeAgo } from './time';
	import { game, DIFFICULTY_META, DIFFICULTY_LABELS } from './game.svelte.js';
	import { session } from '$lib/stores/session.svelte';
	import { lobby } from '$lib/stores/lobby.svelte';
	import { net } from '$lib/stores/net.svelte';
	import Shape from './Shape.svelte';
	import { SHAPE_COLORS } from './theme.js';
	import Rules from './Rules.svelte';
	import SignIn from './SignIn.svelte';
	import NetBadge from './NetBadge.svelte';

	// Measure *latency only* on landing — fast and tiny, so it never janks the
	// page. Tapping the badge runs the full download/upload test on demand.
	onMount(() => {
		if (net.latency == null) net.measure(false);
	});

	type Section = 'play' | 'online' | 'games' | 'friends' | 'profile';

	let section = $state<Section>('play');
	let mode = $state<GameMode>('stack');
	let difficulty = $state<Difficulty>('chief');
	let opponents = $state(1);
	let showRules = $state(false);
	let copied = $state(false);

	const authed = $derived(session.status === 'authed');

	// Refresh on entry rather than polling — the list is only visible here, and a
	// game's own WebSocket already keeps the table itself live.
	$effect(() => {
		if (section === 'games' && authed) void lobby.loadRunningGames();
	});

	/**
	 * The other seats, in seat order — "Ada, Tee-Noble". AI seats carry no name in
	 * the database (it lives in the Redis snapshot), only a difficulty, so their
	 * label comes from that.
	 */
	function opponentLabel(g: GameSummary): string {
		if (g.opponents.length === 0) return 'No opponents';
		return g.opponents
			.map((o) =>
				o.is_ai
					? (o.ai_difficulty && DIFFICULTY_LABELS[o.ai_difficulty]) || 'AI'
					: (o.display_name ?? o.username ?? 'Player')
			)
			.join(', ');
	}

	async function copyLink() {
		if (!lobby.inviteLink) return;
		try {
			await navigator.clipboard.writeText(lobby.inviteLink);
			copied = true;
			setTimeout(() => (copied = false), 1500);
		} catch {
			/* clipboard blocked — the link is selectable in the field */
		}
	}

	// Add a passkey (email-free account claim / passwordless device).
	let pkBusy = $state(false);
	let pkError = $state<string | null>(null);
	let pkDone = $state(false);
	async function addPasskey() {
		pkBusy = true;
		pkError = null;
		try {
			await session.addPasskey();
			pkDone = true;
			setTimeout(() => (pkDone = false), 2500);
		} catch (e) {
			pkError = e instanceof Error ? e.message : 'Could not add passkey.';
		} finally {
			pkBusy = false;
		}
	}

	// Guest → full account upgrade.
	let upEmail = $state('');
	let upPassword = $state('');
	let upBusy = $state(false);
	let upError = $state<string | null>(null);
	let showUpgrade = $state(false);
	async function upgrade() {
		upBusy = true;
		upError = null;
		try {
			await session.upgradeGuest(upEmail.trim(), upPassword);
			showUpgrade = false;
			upEmail = '';
			upPassword = '';
		} catch (e) {
			upError = e instanceof Error ? e.message : 'Could not upgrade.';
		} finally {
			upBusy = false;
		}
	}

	const NAV: { id: Section; icon: string; label: string }[] = [
		{ id: 'play', icon: '🎴', label: 'Play' },
		{ id: 'online', icon: '🌐', label: 'Online' },
		{ id: 'games', icon: '⏳', label: 'Games' },
		{ id: 'friends', icon: '👥', label: 'Friends' },
		{ id: 'profile', icon: '👤', label: 'Profile' }
	];

	const HERO_SHAPES = ['circle', 'triangle', 'cross', 'square', 'star'] as const;

	function deal() {
		game.start({ mode, difficulty, opponents });
	}
</script>

<div class="menu">
	<div class="net-corner"><NetBadge detail /></div>
	<div class="hero">
		<div class="hero-shapes">
			{#each HERO_SHAPES as s (s)}
				<Shape shape={s} size={26} color={SHAPE_COLORS[s]} />
			{/each}
		</div>
		<h1>WHOT<span class="s">!</span></h1>
		<p class="tag">The Nigerian card classic. Match, disrupt, empty your hand first.</p>
	</div>

	<div class="stage">
		<!-- ── Play (vs AI) ── -->
		{#if section === 'play'}
			<div class="panel">
				<div class="field">
					<span class="label">Mode</span>
					<div class="segmented">
						<button class:on={mode === 'stack'} onclick={() => (mode = 'stack')}>
							Stack<small>Counter & pile on penalties</small>
						</button>
						<button class:on={mode === 'no_stack'} onclick={() => (mode = 'no_stack')}>
							No-stack<small>Penalties resolve at once</small>
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
							<button
								class="diff"
								class:on={difficulty === d.id}
								onclick={() => (difficulty = d.id)}
							>
								<span class="diff-name">{d.label}</span>
								<span class="diff-blurb">{d.blurb}</span>
							</button>
						{/each}
					</div>
				</div>

				<button class="play" onclick={deal} disabled={!authed}>Deal me in</button>
				{#if !authed}<span class="hint center">Sign in on Profile to play.</span>{/if}
				{#if game.error}<span class="err center">{game.error}</span>{/if}
				<button class="rules-link" onclick={() => (showRules = true)}>How to play</button>
			</div>

			<!-- ── Online (matchmaking + rooms) ── -->
		{:else if section === 'online'}
			<div class="panel">
				{#if !authed}
					<p class="signin-nudge">Sign in on <strong>Profile</strong> to play online.</p>
				{:else if lobby.inQueue}
					<div class="queue">
						<span class="spinner-sm"></span>
						<span>Finding an opponent…</span>
						<button class="linkbtn" onclick={() => lobby.cancelMatch()}>Cancel</button>
					</div>
				{:else}
					<div class="field">
						<span class="label">Quick match</span>
						<button class="online" onclick={() => lobby.findMatch(mode)}>Find a match</button>
						<span class="hint">A random opponent, 1-on-1, {mode === 'stack' ? 'Stack' : 'No-stack'} mode.</span>
					</div>
					<div class="field">
						<span class="label">Private room</span>
						<button class="online room" onclick={() => lobby.createRoom(mode)}>Create room</button>
						<span class="hint">Invite friends and add AI seats — up to 6 at the table.</span>
					</div>
				{/if}
				{#if lobby.error}<span class="err">{lobby.error}</span>{/if}
			</div>

			<!-- ── Friends ── -->
			<!-- ── Games in progress ── -->
		{:else if section === 'games'}
			<div class="panel">
				{#if !authed}
					<p class="signin-nudge">Sign in on <strong>Profile</strong> to see your games.</p>
				{:else}
					<div class="field">
						<span class="label">In progress</span>
						{#if lobby.runningGames.length === 0}
							<p class="empty">
								{lobby.gamesLoading ? 'Loading…' : 'No games running. Start one from Play or Friends.'}
							</p>
						{/if}
						{#each lobby.runningGames as g (g.id)}
							{@const yourTurn = g.current_seat_index === g.seat_index}
							<div class="game-row">
								<span class="who">
									{opponentLabel(g)}
									<small>
										Started {timeAgo(g.created_at)} · {g.mode === 'stack' ? 'Stack' : 'No-stack'}
									</small>
								</span>
								<div class="actions">
									{#if yourTurn}<span class="turn">Your turn</span>{/if}
									<button class="mini go" onclick={() => lobby.resumeGame(g.id)}>Resume</button>
								</div>
							</div>
						{/each}
					</div>
					<button class="online" onclick={() => lobby.loadRunningGames()}>Refresh</button>
				{/if}
				{#if lobby.error}<span class="err">{lobby.error}</span>{/if}
			</div>

			<!-- ── Friends ── -->
		{:else if section === 'friends'}
			<div class="panel">
				{#if !authed}
					<p class="signin-nudge">Sign in on <strong>Profile</strong> to add friends.</p>
				{:else}
					{#if lobby.requests.length > 0}
						<div class="field">
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
						</div>
					{/if}
					<div class="field">
						<span class="label">Your friends</span>
						{#if lobby.friends.length === 0}
							<p class="empty">No friends yet — search below to add someone.</p>
						{/if}
						{#each lobby.friends as f (f.id)}
							<div class="person">
								<span class="who">{f.display_name}<small>@{f.username}</small></span>
								<div class="actions">
									<button class="mini go" onclick={() => lobby.inviteFriend(f, mode)}>Play</button>
									<button class="mini danger" onclick={() => lobby.unfriend(f.username)}>Remove</button>
								</div>
							</div>
						{/each}
					</div>
					<div class="field">
						<span class="label">Invite a friend</span>
						{#if lobby.inviteLink}
							<div class="invite-link">
								<input readonly value={lobby.inviteLink} onclick={(e) => e.currentTarget.select()} />
								<button class="mini go" onclick={copyLink}>{copied ? 'Copied!' : 'Copy'}</button>
							</div>
							<span class="hint">One-time link — send it to someone you know. They become your friend the moment they open it.</span>
						{:else}
							<button class="online" onclick={() => lobby.createInviteLink()}>Create invite link</button>
							<span class="hint">No username search — you add friends only by sharing a one-time link.</span>
						{/if}
					</div>
				{/if}
				{#if lobby.error}<span class="err">{lobby.error}</span>{/if}
			</div>

			<!-- ── Profile ── -->
		{:else}
			<div class="panel">
				{#if authed && session.user}
					<div class="field">
						<span class="label">Signed in</span>
						<div class="who">
							<span>
								<strong>{session.user.display_name}</strong>{#if session.user.is_guest}<em> (guest)</em>{/if}
							</span>
							<button class="linkbtn" onclick={() => session.logout()}>Sign out</button>
						</div>
						{#if session.user.beat_tee_noble}
							<div class="badge-chip">🏆 Tee-Noble Slayer</div>
						{/if}
						{#if session.user.has_passkey}
							<div class="badge-chip pk">🔑 Passkey enabled</div>
						{/if}
						{#if session.passkeysSupported}
							<button class="online" onclick={addPasskey} disabled={pkBusy}>
								{pkBusy
									? '…'
									: pkDone
										? '✓ Passkey saved'
										: session.user.has_passkey
											? '🔑 Add another passkey'
											: '🔑 Add a passkey (no email)'}
							</button>
							{#if pkError}<span class="err">{pkError}</span>{/if}
						{/if}
						{#if session.user.is_guest}
							<span class="hint">You're a guest. Add a passkey above (no email needed), or an email + password, so you can log back in.</span>
							{#if showUpgrade}
								<div class="upgrade">
									<input type="email" placeholder="Email" bind:value={upEmail} disabled={upBusy} />
									<input
										type="password"
										placeholder="Password (min 8)"
										bind:value={upPassword}
										disabled={upBusy}
									/>
									<div class="up-actions">
										<button class="online" onclick={upgrade} disabled={upBusy}
											>{upBusy ? '…' : 'Save account'}</button
										>
										<button class="linkbtn" onclick={() => (showUpgrade = false)}>Cancel</button>
									</div>
									{#if upError}<span class="err">{upError}</span>{/if}
								</div>
							{:else}
								<button class="online" onclick={() => (showUpgrade = true)}>Upgrade account</button>
							{/if}
						{/if}
					</div>
				{:else if session.status === 'loading'}
					<span class="hint">…</span>
				{:else}
					<SignIn />
				{/if}
			</div>
		{/if}
	</div>

	<nav class="dock" aria-label="Main navigation">
		{#each NAV as item (item.id)}
			<button class:on={section === item.id} onclick={() => (section = item.id)}>
				<span class="ico">{item.icon}</span>
				<span class="dlabel">{item.label}</span>
				{#if item.id === 'friends' && lobby.requests.length}
					<span class="ndot">{lobby.requests.length}</span>
				{/if}
			</button>
		{/each}
	</nav>
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
		padding: 2rem 1rem 7rem; /* room for the dock */
		gap: 1.5rem;
	}
	.net-corner {
		position: fixed;
		top: 0.9rem;
		right: 0.9rem;
		z-index: 30;
	}
	.hero {
		text-align: center;
	}
	.hero-shapes {
		display: flex;
		gap: 0.7rem;
		justify-content: center;
		margin-bottom: 0.6rem;
		opacity: 0.95;
	}
	h1 {
		font-size: clamp(2.6rem, 11vw, 4.8rem);
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
		margin: 0.6rem auto 0;
		color: rgba(255, 255, 255, 0.7);
		max-width: 26rem;
		font-size: 0.95rem;
	}

	.stage {
		width: 100%;
		max-width: 520px;
		flex: 1;
	}
	.panel {
		width: 100%;
		background: rgba(10, 22, 16, 0.6);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: 20px;
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1.3rem;
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
		color: rgba(255, 255, 255, 0.5);
		font-style: italic;
	}
	.hint.center {
		text-align: center;
	}
	.signin-nudge {
		text-align: center;
		color: rgba(255, 255, 255, 0.75);
		margin: 0.5rem 0;
	}
	.signin-nudge strong {
		color: var(--gold, #e8b84b);
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
	.who small {
		color: rgba(255, 255, 255, 0.4);
		margin-left: 0.35rem;
	}
	.who em {
		color: rgba(255, 255, 255, 0.4);
		font-style: normal;
		font-size: 0.82rem;
	}

	.online {
		padding: 0.85rem;
		border-radius: 12px;
		border: none;
		background: linear-gradient(135deg, #2f9e6f, #22795a);
		color: #fff;
		font-weight: 800;
		font-size: 0.98rem;
		cursor: pointer;
		transition: filter 0.15s ease, transform 0.24s var(--spring);
	}
	.online.room {
		background: linear-gradient(135deg, #e8b84b, #d99a2b);
		color: #1a1205;
	}
	.online:hover {
		filter: brightness(1.08);
		transform: translateY(-2px);
	}
	.queue {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		padding: 0.75rem;
		border-radius: 12px;
		background: rgba(47, 158, 111, 0.12);
		border: 1px solid rgba(47, 158, 111, 0.3);
		color: rgba(255, 255, 255, 0.85);
		font-size: 0.9rem;
	}
	.spinner-sm {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		border: 2px solid rgba(255, 255, 255, 0.2);
		border-top-color: #7fe0b3;
		animation: spin 0.7s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
	.linkbtn {
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.5);
		text-decoration: underline;
		cursor: pointer;
		font-size: 0.8rem;
	}
	.err {
		font-size: 0.78rem;
		color: #ff8fa3;
	}
	.err.center {
		text-align: center;
	}

	/* friends list */
	.person {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.15rem 0;
	}
	/* Like .person, but each row is a distinct game so it gets a divider. */
	.game-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.55rem 0;
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}
	.game-row:last-child {
		border-bottom: none;
	}
	.turn {
		font-size: 0.68rem;
		font-weight: 800;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--gold, #e8b84b);
		align-self: center;
		white-space: nowrap;
	}
	.actions {
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	.mini {
		border: none;
		border-radius: 7px;
		padding: 0.35rem 0.7rem;
		font-size: 0.78rem;
		font-weight: 700;
		cursor: pointer;
		background: rgba(255, 255, 255, 0.08);
		color: rgba(255, 255, 255, 0.8);
	}
	.mini.go {
		background: #2f9e6f;
		color: #fff;
	}
	.mini.danger {
		background: rgba(255, 84, 112, 0.18);
		color: #ff8fa3;
	}
	.empty {
		color: rgba(255, 255, 255, 0.4);
		font-size: 0.85rem;
		margin: 0;
	}
	.badge-chip {
		align-self: flex-start;
		background: linear-gradient(135deg, #e8b84b, #d99a2b);
		color: #1a1205;
		font-weight: 800;
		font-size: 0.8rem;
		padding: 0.3rem 0.7rem;
		border-radius: 999px;
		box-shadow: 0 3px 12px rgba(232, 184, 75, 0.3);
	}
	.badge-chip.pk {
		background: rgba(47, 158, 111, 0.2);
		color: #7fe0b3;
		box-shadow: none;
	}
	.upgrade {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		margin-top: 0.3rem;
	}
	.upgrade input {
		padding: 0.55rem 0.7rem;
		border-radius: 10px;
		border: 1px solid rgba(255, 255, 255, 0.12);
		background: rgba(255, 255, 255, 0.04);
		color: #fff;
		font-size: 0.88rem;
	}
	.up-actions {
		display: flex;
		align-items: center;
		gap: 0.6rem;
	}
	.up-actions .online {
		padding: 0.5rem 0.9rem;
	}
	.invite-link {
		display: flex;
		gap: 0.4rem;
	}
	.invite-link input {
		flex: 1;
		min-width: 0;
		padding: 0.55rem 0.7rem;
		border-radius: 10px;
		border: 1px solid rgba(255, 255, 255, 0.12);
		background: rgba(255, 255, 255, 0.04);
		color: rgba(255, 255, 255, 0.85);
		font-size: 0.8rem;
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
	.play:hover:not(:disabled) {
		filter: brightness(1.06);
		transform: translateY(-2px);
	}
	.play:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	.rules-link {
		background: none;
		border: none;
		color: rgba(255, 255, 255, 0.55);
		cursor: pointer;
		text-decoration: underline;
		font-size: 0.85rem;
	}

	/* ── Dock / bottom-nav ── */
	.dock {
		position: fixed;
		left: 50%;
		bottom: 1rem;
		transform: translateX(-50%);
		z-index: 40;
		display: flex;
		gap: 0.3rem;
		padding: 0.45rem;
		background: rgba(10, 22, 16, 0.82);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: 20px;
		backdrop-filter: blur(14px);
		box-shadow: 0 12px 44px rgba(0, 0, 0, 0.5);
	}
	.dock button {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		padding: 0.45rem 1rem;
		border: none;
		background: none;
		color: rgba(255, 255, 255, 0.6);
		cursor: pointer;
		border-radius: 14px;
		transition: background 0.18s, color 0.18s;
	}
	.dock .ico {
		font-size: 1.5rem;
		line-height: 1;
		transition: transform 0.18s cubic-bezier(0.2, 1.3, 0.5, 1);
	}
	.dock button:hover .ico {
		transform: translateY(-5px) scale(1.3);
	}
	.dock .dlabel {
		font-size: 0.64rem;
		font-weight: 700;
		letter-spacing: 0.02em;
	}
	.dock button.on {
		color: #fff;
		background: rgba(232, 184, 75, 0.16);
	}
	.ndot {
		position: absolute;
		top: 3px;
		right: 10px;
		min-width: 16px;
		height: 16px;
		padding: 0 4px;
		border-radius: 999px;
		background: #ff5470;
		color: #fff;
		font-size: 0.66rem;
		font-weight: 800;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	@media (max-width: 640px) {
		.dock {
			left: 0;
			right: 0;
			bottom: 0;
			width: 100%;
			transform: none;
			justify-content: space-around;
			gap: 0;
			padding: 0.35rem 0.2rem calc(0.35rem + env(safe-area-inset-bottom, 0px));
			border-radius: 16px 16px 0 0;
		}
		.dock button {
			flex: 1;
			padding: 0.35rem;
		}
		.dock .ico {
			font-size: 1.35rem;
		}
		/* No magnify on touch. */
		.dock button:hover .ico {
			transform: none;
		}
	}
	@media (max-width: 420px) {
		.diffs {
			grid-template-columns: 1fr;
		}
	}
</style>
