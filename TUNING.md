# Whots AI Difficulty Tuner

This document explains the optimization and benchmarking pipeline for the Whots AI difficulty levels.
Run the Rust tuner to find parameter vectors that ensure each difficulty level consistently beats lower tiers, then compile the resulting parameters into the game engine.

---

## Quick Start

The tuner is implemented in Rust (`server/src/bin/tuner.rs`) utilizing parallel rayon simulations.

```bash
# From the repository root (or inside server/):

# One-shot run (500 games/eval, 50 sweeps)
cargo run --release --bin tuner --manifest-path server/Cargo.toml

# Run continuously — restarts automatically on convergence
cargo run --release --bin tuner --manifest-path server/Cargo.toml -- --games 2000 --continuous

# Resume from checkpoint (reads scripts/params/best.json)
cargo run --release --bin tuner --manifest-path server/Cargo.toml -- --resume --continuous

# Verify the current checkpoint without optimizing
cargo run --release --bin tuner --manifest-path server/Cargo.toml -- --verify
```

Press **Ctrl+C** at any time. Progress is continuously written to `scripts/params/best.json` after every improvement.

---

## Available AI Binaries

The `server` crate provides several standalone binaries for AI testing and tuning:

| Binary | Command | Purpose |
|---|---|---|
| `tuner` | `cargo run --release --bin tuner` | Coordinate-descent optimizer for difficulty weights. |
| `ladder` | `cargo run --release --bin ladder` | Simulates a full round-robin tournament across all AI tiers and reports win matrices. |
| `sweep` | `cargo run --release --bin sweep` | Evaluates parameter sweeps across individual AI reasoning modules. |
| `identity` | `cargo run --release --bin identity` | Benchmarks ISMCTS game-state playouts and self-play consistency. |

---

## How Many Iterations Are Enough?

### Empirical Convergence

The optimizer was run at 100 games/eval across 60 allowed sweeps. Results:

| Sweep | Event |
|-------|-------|
| 6     | Global ordering first reached **100%** (all 21 pairs correct) |
| 11    | **Natural convergence** — step decayed below minimum, optimizer stopped |
| 12–60 | No further progress (these sweeps are no-ops) |

Coordinate descent converges in at most `ceil(log(MIN_STEP / INITIAL_STEP) / log(STEP_DECAY))` no-improvement sweeps:

```
0.15 → 0.09 → 0.054 → 0.032 → 0.019 → 0.012 → 0.007 → 0.004 (stop)
       ×0.6   ×0.6    ×0.6    ×0.6    ×0.6    ×0.6    ×0.6
```

### Iterated Local Search: `--continuous`

`--continuous` replaces a fixed sweep limit with iterated local search:

1. Run coordinate descent until convergence (~11 sweeps).
2. Lightly perturb the all-time best parameters (random jitter per parameter).
3. If the perturbed state finds a better local optimum, accept it; otherwise revert to best.
4. Repeat indefinitely.

---

## Checkpoint Format

`scripts/params/best.json` stores the current best parameters:

```json
{
  "meta": {
    "sweeps_completed": 42,
    "games_per_eval": 500,
    "saved_at": "2026-07-04T08:50:00+00:00",
    "best_global_score": 0.98
  },
  "params": {
    "pikin": { ... },
    "smallz": { ... },
    "isabi_small": { ... },
    "chief": { ... },
    "egbon": { ... },
    "jagaban": { ... }
  }
}
```

To reset and start fresh, delete `scripts/params/best.json`.

---

## Shipping Tuned Parameters

Once `--verify` reports `globalScore ≥ 0.95` across simulated games, copy the parameter weights from `scripts/params/best.json` into `default_params` inside `server/src/game/ai/params.rs`.

Then run the tournament ladder to verify win-rate monotonicity:

```bash
cargo run --release --bin ladder --manifest-path server/Cargo.toml
```

Expected output: `tee-noble ≥ jagaban > egbon > chief > isabi_small > smallz > pikin` in win rates.

---

## Parameter Meanings by Tier

| Level | Active Modules | Character |
|---|---|---|
| `pikin` | None (`noise = 100.0`) | Completely random play — baseline / beginner. |
| `smallz` | Hand-thinning | Sheds dominant suits but ignores opponent states. |
| `isabi_small` | + Action-awareness | Plays Pick-Two, Suspension, and Hold-On strategically. |
| `chief` | + Threat-detection | Actively attacks opponents who are close to emptying their hand. |
| `egbon` | + Card-probability, Whot-intelligence, Setup-plays | Estimates deck probability and conserves Whot 20 wild cards. |
| `jagaban` | + Anticipation | Predicts opponent counter-plays and chains multi-turn card plays. |
| `tee-noble` | Information Set MCTS (ISMCTS) | Monte Carlo simulation with near-flawless decision trees. |
