//! Measurement harness for the REAL production difficulty ladder.
//!
//! Unlike the tuner (which measures pure-heuristic play), this uses `policy_for`
//! — the exact policies the game runs (epsilon-greedy for the low rungs, ISMCTS
//! search for Jagaban/Tee-Noble). It prints the true win-rate matrix and sweeps
//! Tee-Noble's search budget vs Jagaban to see whether more search separates them.

use rand::rngs::StdRng;
use rand::SeedableRng;
use rayon::prelude::*;
use whots_server::game::{
    ai::ismcts::{act, apply_ai_move, policy_for, strong_rollout_params, Budget, Policy},
    engine::create_game,
    types::{Difficulty, GameMode, GamePhase, Seat, SeatKind},
};

const LADDER: &[(Difficulty, &str)] = &[
    (Difficulty::Pikin, "pikin"),
    (Difficulty::Smallz, "smallz"),
    (Difficulty::IsabiSmall, "isabiSmall"),
    (Difficulty::Chief, "chief"),
    (Difficulty::Egbon, "egbon"),
    (Difficulty::Jagaban, "jagaban"),
    (Difficulty::TeeNoble, "tee-noble"),
];

fn play(pa: &Policy, pb: &Policy, a_seat: usize, seed: u64) -> Option<bool> {
    let mut rng = StdRng::seed_from_u64(seed);
    let seats = vec![
        Seat { name: "0".into(), kind: SeatKind::Ai { difficulty: Difficulty::Pikin }, hand: vec![], owed_draws: 0 },
        Seat { name: "1".into(), kind: SeatKind::Ai { difficulty: Difficulty::Pikin }, hand: vec![], owed_draws: 0 },
    ];
    let mut state = create_game(seats, GameMode::Stack);
    let mut turns = 0;
    while state.phase == GamePhase::Playing && turns < 600 {
        turns += 1;
        let idx = state.current_seat_index;
        let pol = if idx == a_seat { pa } else { pb };
        let mv = act(&state, idx, pol, &mut rng);
        if apply_ai_move(&mut state, idx, mv).is_err() {
            break;
        }
    }
    state.winner_index.map(|w| w == a_seat)
}

/// Win rate of `hi` vs `lo` over `n` games, alternating who goes first.
fn win_rate(hi: &Policy, lo: &Policy, n: usize, salt: u64) -> f64 {
    let (w, t) = (0..n)
        .into_par_iter()
        .filter_map(|i| play(hi, lo, i % 2, salt.wrapping_add(i as u64)))
        .map(|x| (x as u32, 1u32))
        .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    if t == 0 { 0.5 } else { w as f64 / t as f64 }
}

fn tee(iters: u32) -> Policy {
    Policy::Ismcts {
        budget: Budget::Iterations(iters),
        temperature: 0.0,
        exploration: 1.4,
        rollout: strong_rollout_params(),
        endgame_samples: 200,
    }
}

fn main() {
    let n_ladder: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(150);
    let n_sweep: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(150);

    println!("=== REAL production ladder (policy_for), {n_ladder} games/pair ===");
    print!("{:>11}", "");
    for (_, name) in LADDER {
        print!("{name:>10}");
    }
    println!();
    let mut fails = 0;
    for j in 0..LADDER.len() {
        print!("{:>11}", LADDER[j].1);
        for i in 0..LADDER.len() {
            if i == j {
                print!("{:>10}", "-");
            } else if j > i {
                let r = win_rate(&policy_for(LADDER[j].0), &policy_for(LADDER[i].0), n_ladder, (j * 100 + i) as u64);
                if r <= 0.5 {
                    fails += 1;
                }
                print!("{r:>10.2}");
            } else {
                print!("{:>10}", ".");
            }
        }
        println!();
    }
    println!("inversions (higher wins <= 0.50 vs lower): {fails} / 21");

    println!("\n=== Tee-Noble search-budget sweep vs Jagaban (ISMCTS 512), {n_sweep} games each ===");
    let jag = policy_for(Difficulty::Jagaban);
    for &b in &[512u32, 2048, 8192, 16384] {
        let r = win_rate(&tee(b), &jag, n_sweep, b as u64 * 7);
        println!("  tee-noble @ {b:>6} iters -> {:.1}% vs jagaban", r * 100.0);
    }
}
