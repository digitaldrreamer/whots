# Whots — Game Design Plan

## The Game

Nigerian Whots card game. Players match cards by shape or number, using action cards to disrupt opponents, first to empty their hand wins.

---

## The Deck

- 5 suits (shapes): Circle, Triangle, Cross, Square, Star
- Cards numbered within each suit (not all numbers appear in every suit)
- Whot cards (numbered 20) — wild cards

---

## Rules

### Basic Gameplay

- Deal 5 cards to each player
- Flip one card face-up to start the discard pile
- Remaining cards form the stock pile
- Match the top card by shape or number
- If you can't play, draw from the stock pile
- First to empty their hand wins

### Special Action Cards

| Card | Name | Action |
|------|------|--------|
| 1 | Hold On | Next player skips a turn. Player who played it can immediately follow with another card of the same number or same shape |
| 2 | Pick Two | Next player draws 2 cards |
| 5 | Pick Three | Next player draws 3 cards |
| 8 | Suspension | Next player is suspended |
| 14 | General Market | All other players draw 1 card |
| 20 | Whot | Wild card — play on anything, call any shape |

---

## Game Variations

### Stack Mode
2s, 5s, and same-numbered cards can be stacked. A player hit with a penalty can counter with their own matching card, adding to the total. The penalty accumulates until someone can't counter.

Card 1 (Hold On) can always be stacked in both modes — the player who played it can immediately follow with another card of the same number or same shape.

### No-Stack Mode
No stacking of penalty cards. Each action card resolves immediately against the next player with no countering.

---

## Shuffling

Fisher-Yates shuffle (Knuth shuffle). O(n), unbiased, every permutation equally likely.

```
for i from n-1 down to 1:
    j = random integer where 0 ≤ j ≤ i
    swap(deck[i], deck[j])
```

Never use `sort(() => Math.random() - 0.5)` — biased and unreliable.

---

## Computer Player Difficulties

### Philosophy

One optimal play engine. Difficulty levels control which reasoning modules are active, not random deviation from perfect play. Lower difficulties have specific blind spots — they don't randomly make mistakes, they genuinely can't see certain strategic dimensions.

Every legal move is scored across active modules. The highest score wins.

### Reasoning Modules

| Module | What it does |
|--------|-------------|
| Hand thinning | Play from the suit you have most of |
| Action awareness | Prefer 2s, 5s, 14s when available |
| Threat detection | Target whoever has fewest cards |
| Card probability | Estimate what suits opponents likely hold |
| Whot intelligence | Call the suit opponents probably don't have |
| Anticipation | Predict what suit opponent will need next turn |
| Setup plays | Play a card now to unlock a better card next turn |

### Difficulty Levels

| Level | Modules Active |
|-------|---------------|
| Pikin | None — pure random |
| Smallz | Hand thinning |
| iSabiSmall | + Action awareness |
| Chief | + Threat detection |
| Ẹgbọn Àdúgbò | + Card probability, Whot intelligence |
| Jagaban | + Anticipation, setup plays |

### Decision Framework

Each move is evaluated across three layers:

- **Reason** — evaluate current hand and game state using active modules
- **Action** — score each legal move, pick highest
- **Anticipation** — factor in what opponents will likely do next (Ẹgbọn Àdúgbò and above)

---

## Tee-Noble — The Final Boss

Not a difficulty level. An event.

### Appearance
- Triggers randomly per game session
- Weighted to appear more during win streaks — finds you when you're confident
- Announced with a special UI moment when it enters
- One shot: decline or lose, gone until the next random trigger

### How Tee-Noble Plays
- Perfection: 1.0 — all modules active, no randomness, no blind spots
- Reads session patterns — aware of how you've played across current session
- Never wastes a move — every card either reduces hand or damages opponent position
- Calls Whot specifically to hurt the human player, not just generically optimal

### Reward for Beating Tee-Noble
- Permanent special badge (visible on profile)
- 1 month of Pro

The randomness is intentional — you can't grind for it, you can't prepare. It shows up on its own terms. That makes the badge rare and the win earned.
