# Whot UI screenshots

Captured from the SvelteKit app (`app/`) playing a full single-player game.

| Screen | Preview |
| --- | --- |
| Main menu — mode, opponents, difficulty ladder | ![Menu](menu.png) |
| Your turn — playable cards highlighted, rest kept legible | ![Gameplay](gameplay.png) |
| Opponent's turn — "thinking" indicator, hand stays readable | ![Opponent turn](opponent-turn.png) |
| Game over — result overlay | ![Result](result.png) |

## Hand layout toggle

The player's hand can switch between a compact overlapping fan and a fully
spread-out row (which scrolls to pan when it's wider than the screen).

| Held | Spread |
| --- | --- |
| ![Held](hand-held.png) | ![Spread](hand-spread.png) |

## Gamification

Action cards fire a punchy callout banner, penalties shake the board, and a win
sets off a confetti burst — all backed by a procedural (Web Audio) sound engine
with a mute toggle.

| Action callout | Win |
| --- | --- |
| ![Action callout](action-callout.png) | ![Win confetti](win-confetti.png) |
