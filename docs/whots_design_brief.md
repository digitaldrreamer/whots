# Whots — Complete Design Brief

> Every view, state, overlay, and screen the app needs, derived from the full codebase (SvelteKit frontend, Rust/Axum backend, WebSocket protocol, game engine, AI system, Tee-Noble mechanics).

---

## 1. Onboarding & Auth

### 1.1 Landing / Welcome Screen
- **Purpose**: First thing a new user sees. Introduce the game, funnel into guest or registered play.
- **Content**: Game branding, brief tagline, two primary CTAs — "Play as Guest" and "Create Account" — plus a "Log In" link for returning users.
- **Edge cases**: If the user already has a valid refresh token in local storage, skip this entirely and go to Home.

### 1.2 Guest Entry
- **Purpose**: Lowest-friction path into the game. Only requires a username.
- **Content**: Single text input for username (3–30 chars, alphanumeric + underscore/hyphen). Submit button.
- **Validation**: Real-time format validation. Server returns `409 Conflict` if username is taken — show inline error.
- **On success**: Receives `access_token`, `refresh_token`, and `PublicUser`. Store tokens, navigate to Home.

### 1.3 Registration
- **Purpose**: Full account creation for persistent identity, friends, stats.
- **Content**: Username, email, and password fields. Password minimum 8 chars.
- **Validation**: Username format (same as guest), valid email, password length. Server returns `409` on duplicate username or email.
- **On success**: Same token flow as guest. Server sends a verification email in the background. Navigate to Home, optionally show a "verify your email" banner.

### 1.4 Login
- **Purpose**: Returning registered users.
- **Content**: Single "identifier" field (accepts username or email) plus password. "Forgot password?" link.
- **Validation**: Server returns `401 Unauthorized` on bad credentials — show generic "invalid credentials" error (don't leak whether the account exists).

### 1.5 Forgot Password
- **Purpose**: Initiate password reset.
- **Content**: Email input field. Submit button.
- **Behavior**: Always shows "If an account exists, we sent a reset link" regardless of whether the email is found (anti-enumeration). Navigate to a confirmation screen.

### 1.6 Reset Password
- **Purpose**: Set a new password via token from email link.
- **Content**: New password field (min 8 chars), confirm password field. The reset token comes from a URL parameter.
- **On success**: All existing sessions are invalidated. Show confirmation, redirect to Login.
- **Edge cases**: Expired or already-used token → show "invalid or expired link" error.

### 1.7 Email Verification (deep link destination)
- **Purpose**: Confirm email ownership. User arrives from a link in their email.
- **Content**: Minimal — either a success message ("Email verified!") or an error ("Link expired").
- **Behavior**: Calls `POST /auth/verify-email` with the token from the URL. Not a screen you navigate to from within the app.

---

## 2. Home & Navigation

### 2.1 Home Screen
- **Purpose**: Central hub after auth. Quick access to play, social, and history.
- **Content**:
  - Primary CTA: "Play" (opens game setup)
  - Secondary CTA: "Find Match" (online matchmaking)
  - Active game card (if the user has an in-progress game — resume it)
  - Recent games summary (last few from `GET /users/me/games`)
  - Notification badge count (from `GET /notifications/count`)
  - Quick access to Friends
- **Edge cases**: Guest users should see a subtle "upgrade to full account" prompt. If there's a pending Tee-Noble challenge, show the Tee-Noble entrance (see §8).

### 2.2 Shell / Navigation Layout
- **Purpose**: Persistent navigation frame wrapping all authenticated screens.
- **Content**: Bottom tab bar or sidebar with: Home, Friends, Profile. Notification bell icon with unread badge. The layout hosts the notification WebSocket connection (`/ws/notify`) for real-time updates.
- **Behavior**: Notification WS connects on mount, receives live notifications, updates badge count.

---

## 3. Game Setup

### 3.1 Play Mode Selection
- **Purpose**: Choose between playing vs AI or playing with friends/online.
- **Content**: Three options:
  1. **vs Computer** — pick difficulty and start immediately
  2. **vs Friends** — invite specific friends to a private game
  3. **Find Match** — enter the online matchmaking queue

### 3.2 Solo Game Setup (vs Computer)
- **Purpose**: Configure a game against AI opponents.
- **Content**:
  - Number of AI opponents (1–5, since max 6 seats total)
  - Difficulty selector per opponent: Pikin, Smallz, iSabiSmall, Chief, Ẹgbọn Àdúgbò, Jagaban
  - Game mode toggle: Stack / No-Stack (with brief explanation of each)
  - "Start Game" button
- **Behavior**: Calls `POST /games` with the human player seat + AI seats. On success, navigates to the Game Board and opens the WebSocket.

### 3.3 Friend Game Setup (vs Friends)
- **Purpose**: Create a private game and invite friends.
- **Content**:
  - Friend picker: search/select from friend list (`GET /friends`)
  - Optional AI seats to fill remaining slots
  - Game mode toggle: Stack / No-Stack
  - "Send Invites & Start" button
- **Behavior**: Calls `POST /games` with human + AI seats. Server sends `game_invite` notifications to invited friends. Creator enters the Game Board; invited friends see the invite in their notifications.
- **Edge cases**: If a friend declines, the game is abandoned (`POST /games/:id/decline` sets status to `abandoned`). Need to handle this gracefully if the creator is already on the Game Board.

### 3.4 Game Invite (received)
- **Purpose**: A friend invited you to a game.
- **Content**: Shows who invited you, game mode. Two buttons: Accept / Decline.
- **Behavior**: Accept → `POST /games/:id/accept`, then navigate to Game Board. Decline → `POST /games/:id/decline`, game is abandoned, creator is notified.
- **Surface**: Can appear as a notification overlay, a dedicated screen from the notification tap, or an inline action in the notifications list.

### 3.5 Matchmaking Queue
- **Purpose**: Wait for an online opponent.
- **Content**:
  - Game mode selector (Stack / No-Stack) — chosen before joining
  - "Searching for opponent..." state with animation
  - Cancel button (`DELETE /matchmaking/queue`)
  - Queue status polling or notification-based (`match_found` notification via WS)
- **Behavior**: `POST /matchmaking/join` → if `matched: true`, immediately navigate to Game Board. If `matched: false`, show waiting state. When `match_found` notification arrives via WebSocket, navigate to the Game Board.
- **Edge cases**: User closes the app while in queue — they remain in queue until timeout or next visit. On re-entry, check `GET /matchmaking/status` and either resume waiting or clear.

---

## 4. Core Gameplay

### 4.1 Game Board (main play surface)
- **Purpose**: The actual card game. This is the most complex screen.
- **Content**:
  - **Player's hand**: Fan of cards at the bottom, interactive — tap to select, tap again or drag to play
  - **Discard pile**: Center of the board, shows the top card prominently with the current effective shape/number
  - **Stock pile**: Face-down draw pile, tappable when it's your turn and you have no valid moves (or must accept a pending pick penalty)
  - **Opponent areas**: For each other player — show card count (backs only), name, avatar, difficulty badge if AI
  - **Turn indicator**: Clear visual for whose turn it is; highlight the active player
  - **Pending effect indicator**: When there's a pending pick (2 or 5 stacking), show the accumulated penalty total prominently. When a hold-on or suspension is in effect, indicate it
  - **Game mode badge**: Stack or No-Stack, visible but not prominent
  - **Chat button**: Opens in-game text chat (for multiplayer human games)
  - **Video/audio call UI**: WebRTC controls for friends-only games (offer/answer/ICE signaling via the game WS)
  - **Menu/exit button**: Abandon game option with confirmation

### 4.2 Card Play Interaction States
- **Your turn, cards playable**: Highlight valid cards in hand. Non-valid cards are dimmed or unresponsive. Tapping a valid card plays it.
- **Your turn, no cards playable**: Hand is dimmed. Stock pile pulses or highlights, prompting the player to draw.
- **Your turn, pending pick penalty**: If stack mode and you have a 2 or 5, those are highlighted (you can counter). Otherwise, stock pile is highlighted to accept the penalty draw.
- **Not your turn**: Hand is visible but non-interactive. Show a waiting/watching state.
- **Opponent's turn (AI)**: Brief delay (800ms per the code), then the AI plays. Animate the AI's card from their area to the discard pile.
- **Hold On follow-up**: After playing a 1, the same player gets another turn immediately. The UI must clearly communicate that the player goes again.

### 4.3 Whot Card Shape Selector
- **Purpose**: When playing a Whot card (20), the player must choose which shape to call.
- **Content**: Overlay or modal with the 5 shapes: Circle, Triangle, Cross, Square, Star. Each as a large tappable icon.
- **Behavior**: Player taps a shape → the Whot card is played with that `calledShape`. The top card updates to show the Whot with the called shape indicator.

### 4.4 Action Card Effect Animations / Feedback
Each action card needs distinct visual/audio feedback:
- **1 (Hold On)**: "Hold On!" text flash. Turn indicator stays on the same player.
- **2 (Pick Two)**: "+2" badge appears on the next player. If stacking, the total accumulates visually.
- **5 (Pick Three)**: "+3" badge. Same stacking behavior.
- **8 (Suspension)**: "Suspended!" on the next player. Their turn is skipped with a visual indication.
- **14 (General Market)**: "General Market!" — all other players visually receive a card simultaneously.
- **20 (Whot)**: Shape selector appears, then the called shape is prominently displayed on the discard pile.

### 4.5 Game Over Screen
- **Purpose**: Show the result when someone empties their hand.
- **Content**:
  - Winner announcement (name, avatar, "wins!" or "You win!")
  - If you won: celebratory animation
  - If you lost: commiseration, show who won
  - Cards remaining in each player's hand (summary)
  - Buttons: "Play Again" (same setup), "New Game" (back to setup), "Home"
- **Data**: Winner info from the `game_over` WebSocket event (`winner_index`, `winner_name`). Game result saved to DB (`status: 'finished'`, `is_winner` on the winning seat).
- **Tee-Noble trigger**: After this screen, the `afterGame()` function runs on the session state. If Tee-Noble triggers, transition to the Tee-Noble Entrance (§8.1) instead of the normal post-game flow.

### 4.6 Spectator View
- **Purpose**: Watch a game you're not participating in.
- **Content**: Same board layout but ALL hands are hidden (card backs only). No interactive card controls. Chat may still be available.
- **Behavior**: Connected via the same game WebSocket, but the server classifies you as a spectator and sends `make_view(state, None)` — no hand data.

### 4.7 Game Abandoned / Disconnection States
- **Opponent abandoned**: If a human opponent cancels (`DELETE /games/:id`) or declines mid-game, the game status becomes `abandoned`. Show a message: "Your opponent left the game."
- **Connection lost**: WebSocket drops. Show a reconnection overlay with retry. On reconnect, the WS handler sends the current game state snapshot immediately.
- **Game not found**: If the game was cleaned up from Redis, show an error and navigate back to Home.

---

## 5. Social

### 5.1 Friends List
- **Purpose**: View and manage friends.
- **Content**: List of accepted friends with avatar, username, display name, online status (if tracked), friendship date. Each friend has actions: "Challenge" (start a game), "Remove".
- **Data**: `GET /friends`

### 5.2 Friend Requests (Incoming)
- **Purpose**: View and act on pending friend requests.
- **Content**: List of users who sent you a request. Each entry: avatar, username. Buttons: Accept / Decline.
- **Data**: `GET /friends/requests`
- **Behavior**: Accept → `POST /friends/request/:username/accept`. Decline → `POST /friends/request/:username/decline`.

### 5.3 Add Friend / User Search
- **Purpose**: Find and add new friends.
- **Content**: Search input (min 2 chars). Results list showing matching users with "Add Friend" button.
- **Data**: `GET /users/search?q=...` (prefix match on username, excludes guests, limit 20).
- **Behavior**: Tap "Add Friend" → `POST /friends/request/:username`. Show confirmation. Recipient gets a `friend_request` notification.
- **Edge cases**: Can't add yourself. Duplicate request is silently ignored (ON CONFLICT DO NOTHING).

### 5.4 Contact Discovery (future/native)
- **Purpose**: Find friends from phone contacts.
- **Content**: Permission prompt to access contacts. After upload, show matched users with "Add Friend" buttons.
- **Data**: `POST /users/contacts/upload` (sends SHA-256 hashes of E.164 numbers), then `GET /users/contacts/matches` (bidirectional match).
- **Note**: Deferred for PWA — endpoint exists but the client doesn't call it yet. Design should account for it as a future feature.

### 5.5 User Profile (other user)
- **Purpose**: View another player's public profile.
- **Content**: Avatar, username, display name, guest badge if applicable. Friend status (add/pending/friends). Option to challenge to a game.
- **Data**: `GET /users/:username`

---

## 6. Notifications

### 6.1 Notification Center
- **Purpose**: View all notifications.
- **Content**: Scrollable list, newest first (limit 50). Each notification has: type icon, message text, timestamp, read/unread indicator. "Mark all read" action.
- **Notification types** (from the codebase):
  - `game_invite` — "{username} invited you to a game" → tappable, goes to Game Invite (§3.4)
  - `game_accepted` — "{username} accepted your game invite"
  - `game_declined` — "{username} declined your game invite"
  - `match_found` — "Match found! Game is ready" → tappable, goes to Game Board
  - `friend_request` — "{username} sent you a friend request" → tappable, goes to Friend Requests
  - `friend_accepted` — "{username} accepted your friend request"
- **Data**: `GET /notifications`, `PATCH /notifications/:id` (mark one read), `DELETE /notifications` (mark all read).

### 6.2 Real-time Notification Toast
- **Purpose**: Alert the user to new notifications without leaving their current screen.
- **Content**: Slide-in toast/banner at the top with notification summary. Tappable to navigate to the relevant screen. Auto-dismiss after a few seconds.
- **Behavior**: Driven by the `/ws/notify` WebSocket. On connect, server flushes all unread notifications. New ones arrive in real-time.

---

## 7. Profile & Settings

### 7.1 My Profile
- **Purpose**: View and edit your own profile.
- **Content**: Avatar (from DiceBear, seed = username), display name (editable, 1–50 chars), username (read-only), email (read-only, with verification status), guest badge if applicable.
- **Data**: `GET /users/me`, `PUT /users/me` (update display_name, avatar_url).
- **Guest upgrade prompt**: If `is_guest: true`, show a prominent "Create full account" CTA that leads to a registration flow (adds email + password to the existing account).

### 7.2 Game History
- **Purpose**: Review past games.
- **Content**: Paginated list (20 per page) of games with: date, mode (stack/no-stack), result (win/loss), player count, status (finished/abandoned).
- **Data**: `GET /users/me/games?page=N`
- **Tappable**: Each game could expand to show seat details (who played, who won) via `GET /games/:id`.

### 7.3 Settings
- **Purpose**: App preferences.
- **Content**: Sound effects toggle, music toggle, notification preferences, theme (if applicable). "Resend verification email" button if email is unverified (`POST /auth/resend-verification`). Logout button (`DELETE /auth/logout`).

### 7.4 Tee-Noble Badge (on profile)
- **Purpose**: Display the permanent badge earned by beating Tee-Noble.
- **Content**: Special badge/icon on the player's profile, visible to other users.
- **Data**: Tracked via `TeeNobleSession.hasWonBefore`. Needs to be persisted server-side (currently client-side session state — may need a `badges` table or a flag on the user).

---

## 8. Tee-Noble Boss Event

### 8.1 Tee-Noble Entrance
- **Purpose**: The dramatic moment when Tee-Noble appears. This is an EVENT, not a menu option.
- **Trigger**: After a regular game ends, `afterGame()` runs. Based on win streak and probability (5% base + 5% per streak win, max 30%, 90% reduction after first win, 3-game cooldown), it may set status to `pending`.
- **Content**: Full-screen dramatic reveal. Special animation/visual treatment. Tee-Noble's identity/persona. The stakes: "Beat me and earn a permanent badge + 1 month of Pro." Two buttons: "Accept Challenge" / "Walk Away".
- **Behavior**: Accept → `acceptChallenge()`, transition to Game Board with Tee-Noble as the opponent. Decline → `declineChallenge()`, Tee-Noble disappears, return to normal post-game flow.
- **Key design note**: This should feel cinematic. It's not a regular game prompt — it's a boss encounter that found you.

### 8.2 Tee-Noble Game Board
- **Purpose**: The actual game against Tee-Noble. Same board as §4.1 but with unique visual treatment.
- **Content**: Same core gameplay, but:
  - Tee-Noble has a distinct visual identity (special avatar, name treatment)
  - Elevated visual stakes — different color scheme, ambient effects, tension-building elements
  - Tee-Noble plays at perfection 1.0: all reasoning modules active, no noise, no blind spots
  - Tee-Noble specifically targets the human player (reads session patterns)
- **Mode**: The game mode should match the player's recent preference or default to stack.

### 8.3 Tee-Noble Result
- **Purpose**: Outcome of the boss fight.
- **Won**: Major celebration. Badge awarded ("Tee-Noble Conqueror" or similar). Pro subscription activation. This is a rare, earned achievement — the screen should feel monumental.
- **Lost**: Dignified defeat. "Tee-Noble walks away." No shame, no retry button — they'll have to wait for the next random appearance. Encouragement to keep playing.
- **Behavior**: `resolveChallenge(session, 'won' | 'lost')`. Status becomes `resolved`. `hasWonBefore` is set if won. Win streak continues on win, resets on loss.

---

## 9. Transient / System States

### 9.1 Loading States
- Every screen that fetches data needs a loading skeleton or spinner state.
- Game Board initial load: show a "setting up the table" animation while the WebSocket connects and the first `game_state` event arrives.

### 9.2 Error States
- Network errors: "Can't reach the server. Check your connection." with retry.
- Auth token expired: Silently attempt refresh (`POST /auth/refresh`). If refresh fails, redirect to Login.
- Rate limited: Auth routes are limited to 5 req/min, game routes to 10 req/min. Show "Too many requests, please wait."

### 9.3 Empty States
- No friends yet → "Add friends to play together" with search CTA.
- No game history → "Play your first game!" with play CTA.
- No notifications → "All caught up!" message.
- Matchmaking queue empty / long wait → "Still looking..." with option to cancel and play vs AI.

### 9.4 Confirmation Dialogs
- Abandon game: "Are you sure? This game will be recorded as abandoned."
- Remove friend: "Remove {username} from friends?"
- Logout: "You'll need to log in again."
- Guest account warning (if applicable): "Guest accounts can't be recovered if you log out."

---

## Summary: Complete Screen Inventory

| # | Screen / State | Section |
|---|---------------|---------|
| 1 | Landing / Welcome | §1.1 |
| 2 | Guest Entry | §1.2 |
| 3 | Registration | §1.3 |
| 4 | Login | §1.4 |
| 5 | Forgot Password | §1.5 |
| 6 | Reset Password | §1.6 |
| 7 | Email Verification (deep link) | §1.7 |
| 8 | Home | §2.1 |
| 9 | Navigation Shell | §2.2 |
| 10 | Play Mode Selection | §3.1 |
| 11 | Solo Game Setup | §3.2 |
| 12 | Friend Game Setup | §3.3 |
| 13 | Game Invite (received) | §3.4 |
| 14 | Matchmaking Queue | §3.5 |
| 15 | Game Board | §4.1 |
| 16 | Card Play States (your turn / waiting / pending) | §4.2 |
| 17 | Whot Shape Selector | §4.3 |
| 18 | Action Card Effects | §4.4 |
| 19 | Game Over | §4.5 |
| 20 | Spectator View | §4.6 |
| 21 | Abandoned / Disconnection | §4.7 |
| 22 | Friends List | §5.1 |
| 23 | Friend Requests | §5.2 |
| 24 | Add Friend / Search | §5.3 |
| 25 | Contact Discovery | §5.4 |
| 26 | Other User Profile | §5.5 |
| 27 | Notification Center | §6.1 |
| 28 | Notification Toast | §6.2 |
| 29 | My Profile | §7.1 |
| 30 | Game History | §7.2 |
| 31 | Settings | §7.3 |
| 32 | Tee-Noble Badge Display | §7.4 |
| 33 | Tee-Noble Entrance | §8.1 |
| 34 | Tee-Noble Game Board | §8.2 |
| 35 | Tee-Noble Result | §8.3 |
| 36 | Loading States | §9.1 |
| 37 | Error States | §9.2 |
| 38 | Empty States | §9.3 |
| 39 | Confirmation Dialogs | §9.4 |

**Total: 39 distinct views/states** that the design must account for.
