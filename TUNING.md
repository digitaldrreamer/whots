# Whots AI Difficulty Tuner

This document explains the continuous optimization pipeline for the Whots AI difficulty levels.
Run the tuner to find parameter vectors that make each difficulty level beat the one below it,
then ship the resulting `best.json` values as the game's static AI configuration.

---

## Quick start

```bash
# From the app/ directory
cd app

# One-shot run (500 games/eval, 50 sweeps, stack mode)
npx tsx scripts/tune.ts

# Run continuously for hours — restarts automatically on convergence
npx tsx scripts/tune.ts --games 2000 --continuous

# Resume after an interruption (reads scripts/params/best.json)
npx tsx scripts/tune.ts --resume --continuous

# Just verify the current checkpoint without optimizing
npx tsx scripts/tune.ts --verify
```

Press **Ctrl+C** at any time. Progress is written to `scripts/params/best.json` after every
improvement, so nothing is lost.

---

## How many iterations are enough?

### Empirical convergence (measured)

The optimizer was run at 100 games/eval across 60 allowed sweeps. Results:

| Sweep | Event |
|-------|-------|
| 6     | Global ordering first hit **100%** (all 21 pairs correct) |
| 11    | **Natural convergence** — step decayed below minimum, optimizer stopped |
| 12–60 | No further progress (these sweeps are no-ops) |

The key insight: **setting `--sweeps` above ~15 buys you nothing** with coordinate descent.
The optimizer converges in at most `ceil(log(MIN_STEP / INITIAL_STEP) / log(STEP_DECAY))` no-improvement sweeps:

```
0.15 → 0.09 → 0.054 → 0.032 → 0.019 → 0.012 → 0.007 → 0.004 (stop)
       ×0.6   ×0.6    ×0.6    ×0.6    ×0.6    ×0.6    ×0.6
```

**Maximum useful sweeps per restart: ~15–20** (the rest is wasted wall time).

### What to do instead: `--continuous`

`--continuous` replaces the sweep limit with iterated local search:

1. Run coordinate descent until convergence (~11 sweeps)
2. Lightly perturb the all-time best params (±0.25 random jitter per parameter)
3. Evaluate the perturbed starting point as the new local baseline
4. Run coordinate descent again from there
5. If a new all-time best is found, save to checkpoint
6. Repeat until Ctrl+C

Each restart takes ~6–11 sweeps. With `--games 500` (~34 s/sweep) that is **~3–6 minutes per restart**,
so over 8 hours you get **~80–160 restarts** and the optimizer has thoroughly explored the space around
the best known solution.

### Recommended `--sweeps` per restart

The default `--sweeps 50` is fine — the optimizer exits early anyway. For `--continuous`, `--sweeps`
sets the maximum sweeps *per restart window* (a safety net in case improvements keep coming for a long
time). The default 50 is more than enough.

### Speed at different game counts

| `--games` | Seconds/sweep | Restarts/hour | Games/hour |
|-----------|--------------|---------------|------------|
| 100       | ~4 s         | ~60           | ~32 M      |
| 500       | ~34 s        | ~10           | ~6 M       |
| 2 000     | ~137 s       | ~2.5          | ~6 M       |
| 5 000     | ~340 s       | ~1            | ~6 M       |

The games/hour is roughly constant because `--games` scales both the signal quality and the time.
**Recommendation: `--games 500 --continuous`** for a good balance. Use `--games 2000` if you want
cleaner statistics and can afford fewer restarts.

---

## What the tuner optimizes

Each AI difficulty level is controlled by a `DifficultyParams` vector — nine continuous numbers
instead of the old binary module-on/off switches:

| Parameter          | Range   | Meaning |
|--------------------|---------|---------|
| `handThinning`     | 0 – 4   | Weight for the hand-thinning module (prefer plays that reduce hand size most) |
| `actionAwareness`  | 0 – 4   | Weight for action-awareness (prefer using action cards like Pick-Two when opponent has few cards) |
| `threatDetection`  | 0 – 4   | Weight for threat-detection (block opponents who are about to win) |
| `cardProbability`  | 0 – 4   | Weight for card-probability (prefer shapes depleted from the deck — harder for opponents to match) |
| `whotIntelligence` | 0 – 4   | Weight for whot-intelligence (when forced to play Whot, call the least common shape) |
| `setupPlays`       | 0 – 4   | Weight for setup-plays (prefer plays that leave you with follow-up moves; penalise stranding) |
| `anticipation`     | 0 – 4   | Weight for anticipation (prefer shapes the next player is unlikely to hold) |
| `noise`            | 0 – 200 | Gaussian noise σ added to every candidate score — high values = nearly random play |
| `bluffRate`        | 0 – 0.5 | Probability of deliberately choosing the 2nd-best move (unpredictability) |

The tuner applies **coordinate descent**: for each of the five tunable levels
(`smallz`, `isabiSmall`, `chief`, `egbon`, `jagaban`) it tries ±step for every parameter,
keeps changes that improve the objective, then shrinks the step and repeats.

`pikin` (pure random, `noise=100`) and `tee-noble` (the hardest preset) are fixed anchors
and are never modified.

---

## What "optimal" means

### Primary objective — strict win-rate ladder

In head-to-head 2-player games each higher difficulty must beat each lower difficulty
by a meaningful margin. Specifically, each adjacent pair should satisfy:

```
winRate(higher vs lower) > 0.52
```

### Global ordering score

The tuner tracks a **global ordering score**: the fraction of all C(7,2) = 21 level
pairs that are in the correct order (higher beats lower). The target is:

```
globalScore ≥ 0.95   →   at least 20 of 21 pairs correct
```

A score of 1.0 means every level beats every level below it. The checkpoint stores the
best global score seen so far.

### Secondary signal — quick objective

During each sweep the tuner uses a **quick objective** (adjacent-level pairs only) for
speed. A candidate param change is accepted only if:

1. The quick objective improves by ≥ 0.001 (noise filter), AND
2. The full global ordering score does not decrease (anti-regression guard)

---

## Checkpoint file format

`scripts/params/best.json` stores the current best parameters:

```json
{
  "meta": {
    "sweepsCompleted": 12,
    "gamesPerEval": 500,
    "savedAt": "2026-06-19T10:30:00.000Z",
    "bestGlobalScore": 0.952
  },
  "params": {
    "pikin":      { "handThinning": 0, "noise": 100, ... },
    "smallz":     { "handThinning": 1.2, "noise": 0.1, ... },
    "isabiSmall": { ... },
    "chief":      { ... },
    "egbon":      { ... },
    "jagaban":    { ... },
    "tee-noble":  { ... }
  }
}
```

To reset and start from scratch, delete `scripts/params/best.json`.

---

## Running millions of games — recommended workflow

| Phase | Command | Purpose |
|-------|---------|---------|
| **1. Explore** | `--games 100 --continuous` | Fast exploration, many restarts, noisy signal |
| **2. Refine** | `--games 500 --continuous --resume` | Cleaner signal, ~10 restarts/hour |
| **3. Polish** | `--games 2000 --continuous --resume` | Low-variance statistics, ~2–3 restarts/hour |
| **4. Verify** | `--games 10000 --verify` | Confirm global ordering holds at high sample size |

Each phase resumes from the previous checkpoint. A complete run typically needs
**2–10 million games** to reach a stable optimum. At ~1 300 games/second that's
**30 minutes to 2 hours** of wall time for a meaningful result.

> **Do not use `--sweeps` with large values.** Coordinate descent converges in ~11 sweeps
> no matter what you set. `--sweeps` only matters per restart window; the default (50) is fine.
> Use `--continuous` to keep running, not a bigger `--sweeps` number.

To run unattended overnight:

```bash
cd app
nohup npx tsx scripts/tune.ts --games 500 --continuous --resume >> tune.log 2>&1 &
tail -f tune.log
```

To stop and inspect progress at any time:

```bash
# Kill the background job
kill %1

# Verify what's in the checkpoint
npx tsx scripts/tune.ts --verify
```

---

## Shipping the tuned params

Once `--verify` reports `globalScore ≥ 0.95` across at least 5 000 games per pair,
copy the values from `scripts/params/best.json` into `DEFAULT_PARAMS` in
`src/lib/game/computer/params.ts`. That file is imported at build time — no JSON loading
at runtime.

Example update (edit `DEFAULT_PARAMS` in `params.ts`):

```typescript
// Replace the hand-written defaults with tuned values
export const DEFAULT_PARAMS = {
  pikin:      { ...BASE, noise: 100 },          // always fixed
  smallz:     { ...BASE, handThinning: 1.24 },  // from best.json
  // ...
  'tee-noble': { ...BASE, handThinning: 1, ... , noise: 0.02 } // always fixed
};
```

Then run the existing simulation to sanity-check:

```bash
npx tsx scripts/simulate.ts
```

Expected output: `jagaban > egbon > chief > isabiSmall > smallz > pikin` in win rates,
with `tee-noble` at or above `jagaban`.

---

## Adding new metrics / tendencies

To add a new reasoning dimension:

1. **Create the scoring function** in `src/lib/game/computer/modules/your-module.ts`.
   It must implement `ScoringModule: (candidate, ctx) => number`.
   Score range should be comparable to existing modules (roughly 0–20 before weighting).

2. **Add it to `scoreCandidateWithParams`** in `src/lib/game/computer/params.ts`:
   ```typescript
   yourModule(candidate, ctx) * p.yourModule +
   ```

3. **Add the parameter key** to `DifficultyParams`, `PARAM_KEYS`, and `PARAM_BOUNDS`.

4. **Set initial values** in `DEFAULT_PARAMS` (usually 0 for low levels, 1 for high).

5. **Re-run the tuner** from scratch (delete `best.json`).

The tuner will automatically explore the new parameter alongside the existing ones.

---

## Parameter meanings at each default level

| Level      | Modules active | Character |
|------------|---------------|-----------|
| `pikin`    | none (noise=100) | Completely random — children learning the game |
| `smallz`   | hand-thinning | Plays to shed cards but ignores opponents |
| `isabiSmall` | + action-awareness | Starts using Pick-Two / Suspension strategically |
| `chief`    | + threat-detection | Blocks opponents who are close to winning |
| `egbon`    | + card-probability, whot-intelligence, setup-plays | Thinks about deck composition; conserves Whot cards |
| `jagaban`  | + anticipation | Anticipates what the next player can match |
| `tee-noble` | same as jagaban but noise≈0 | Near-deterministic best-play |

---

## Interpreting tuner output

```
Sweep 3/50  step=0.15
  chief: threatDetection 1.00 → 1.15  quickObj +0.023  ✓ global 0.905 → 0.919
  chief: actionAwareness 1.00 → 0.85  quickObj -0.001  ✗ reverted
  ...
Sweep 3 done  globalScore=0.919  (no overall improvement, step → 0.09)

Global ordering matrix (row beats column):
            pikin  smallz  isabi  chief  egbon  jagaban  tee-noble
pikin         -    0.38    0.35   0.31   0.28    0.26     0.25
smallz       0.62    -     0.46   0.44   0.37    0.34     0.33
...
```

Each row shows that level's win rate against each column. An entry below 0.50 means
the row loses to the column — which is correct when the column is a higher difficulty.
An entry above 0.50 where the row is *lower* difficulty indicates a violation (marked
automatically when `globalScore < 1.0`).
