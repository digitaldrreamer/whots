//! Ladder calibration harness for the ISMCTS difficulty system.
//!
//!   cargo run --release --bin ladder -- matrix 2000     # full 7x7
//!   cargo run --release --bin ladder -- pair tee-noble pikin 2000
//!   cargo run --release --bin ladder -- curve 2000      # Elo-vs-budget sanity
//!
//! Uses the production `select_move` dispatch, so it measures the real ladder.

use std::time::Instant;

use rand::rngs::StdRng;
use rand::SeedableRng;

use whots_server::game::{
    ai::ismcts::{act, policy_for, strong_rollout_params, Budget, Policy},
    engine::{apply_action, create_game},
    types::{Action, Difficulty, GameMode, GamePhase, Seat, SeatKind},
};

const MAX_TURNS: usize = 600;
const TARGET: f64 = 0.559;

const LADDER: &[Difficulty] = &[
    Difficulty::Pikin,
    Difficulty::Smallz,
    Difficulty::IsabiSmall,
    Difficulty::Chief,
    Difficulty::Egbon,
    Difficulty::Jagaban,
    Difficulty::TeeNoble,
];

fn name(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Pikin => "pikin",
        Difficulty::Smallz => "smallz",
        Difficulty::IsabiSmall => "isabiSmall",
        Difficulty::Chief => "chief",
        Difficulty::Egbon => "egbon",
        Difficulty::Jagaban => "jagaban",
        Difficulty::TeeNoble => "tee-noble",
    }
}

fn parse(s: &str) -> Option<Difficulty> {
    LADDER.iter().copied().find(|&d| name(d) == s)
}

fn seat(d: Difficulty) -> Seat {
    Seat { name: name(d).into(), kind: SeatKind::Ai { difficulty: d }, hand: vec![] }
}

/// One heads-up game; returns Some(true) if `a` won. `a_first` alternates seats.
fn play(a: Difficulty, b: Difficulty, a_first: bool, rng: &mut StdRng) -> Option<bool> {
    let (s0, s1) = if a_first { (a, b) } else { (b, a) };
    let p0 = policy_for(s0);
    let p1 = policy_for(s1);
    let mut state = create_game(vec![seat(s0), seat(s1)], GameMode::Stack);
    let mut turns = 0;
    while state.phase == GamePhase::Playing && turns < MAX_TURNS {
        turns += 1;
        let idx = state.current_seat_index;
        let policy = if idx == 0 { &p0 } else { &p1 };
        let action = act(&state, idx, policy, rng);
        if apply_action(&mut state, idx, action).is_err() {
            let _ = apply_action(&mut state, idx, Action::Draw);
        }
    }
    state.winner_index.map(|w| if a_first { w == 0 } else { w == 1 })
}

/// Win rate of `hi` against `lo` over `n` games (seats alternate).
fn win_rate(hi: Difficulty, lo: Difficulty, n: usize) -> f64 {
    let mut rng = StdRng::seed_from_u64(0x9E3779B9 ^ ((hi as u64) << 8) ^ (lo as u64));
    let (mut wins, mut total) = (0usize, 0usize);
    for i in 0..n {
        if let Some(hi_won) = play(hi, lo, i % 2 == 0, &mut rng) {
            if hi_won {
                wins += 1;
            }
            total += 1;
        }
    }
    if total == 0 { 0.5 } else { wins as f64 / total as f64 }
}

fn elo(p: f64) -> f64 {
    let p = p.clamp(0.001, 0.999);
    -400.0 * (1.0 / p - 1.0).log10()
}

fn ismcts(iters: u32, temp: f64) -> Policy {
    Policy::Ismcts {
        budget: Budget::Iterations(iters),
        temperature: temp,
        exploration: 1.4,
        rollout: strong_rollout_params(),
        endgame_samples: 0,
    }
}

/// One heads-up game between two explicit policies.
fn play_pol(p0: &Policy, p1: &Policy, a_first: bool, rng: &mut StdRng) -> Option<bool> {
    let (q0, q1) = if a_first { (p0, p1) } else { (p1, p0) };
    let mut state = create_game(
        vec![seat(Difficulty::TeeNoble), seat(Difficulty::Pikin)],
        GameMode::Stack,
    );
    let mut turns = 0;
    while state.phase == GamePhase::Playing && turns < MAX_TURNS {
        turns += 1;
        let idx = state.current_seat_index;
        let policy = if idx == 0 { q0 } else { q1 };
        let action = act(&state, idx, policy, rng);
        if apply_action(&mut state, idx, action).is_err() {
            let _ = apply_action(&mut state, idx, Action::Draw);
        }
    }
    state.winner_index.map(|w| if a_first { w == 0 } else { w == 1 })
}

fn win_rate_pol(p0: &Policy, p1: &Policy, n: usize, seed: u64) -> f64 {
    let mut rng = StdRng::seed_from_u64(seed);
    let (mut wins, mut total) = (0usize, 0usize);
    for i in 0..n {
        if let Some(w) = play_pol(p0, p1, i % 2 == 0, &mut rng) {
            if w {
                wins += 1;
            }
            total += 1;
        }
    }
    if total == 0 { 0.5 } else { wins as f64 / total as f64 }
}

/// Map raw ISMCTS strength (temp=0) vs a pure-random baseline, and the strength
/// of a few candidate weak policies — so rungs can be picked from real data.
fn grid(n: usize) {
    let random = Policy::Random;
    println!("Strength vs RANDOM baseline, {} games each (temp=0 unless noted):\n", n);
    println!("{:>18}  rate    Elo", "policy");

    let greedy = Policy::Heuristic { epsilon: 0.0, params: strong_rollout_params() };
    let r = win_rate_pol(&greedy, &random, n, 1);
    println!("{:>18}  {:>5.1}%  {:+.0}", "greedy-heuristic", r * 100.0, elo(r));

    for &iters in &[2u32, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096] {
        let r = win_rate_pol(&ismcts(iters, 0.0), &random, n, 100 + iters as u64);
        println!("{:>18}  {:>5.1}%  {:+.0}", format!("ismcts-{iters}"), r * 100.0, elo(r));
    }
}

/// Epsilon-greedy strength vs random — maps the clean bottom-half span.
fn eps_sweep(n: usize) {
    let random = Policy::Random;
    println!("Epsilon-greedy vs RANDOM, {} games each:\n", n);
    println!("{:>10}  rate    Elo", "epsilon");
    for &e in &[1.0f64, 0.85, 0.7, 0.55, 0.4, 0.25, 0.1, 0.0] {
        let p = Policy::Heuristic { epsilon: e, params: strong_rollout_params() };
        let r = win_rate_pol(&p, &random, n, 7000 + (e * 100.0) as u64);
        println!("{:>10.2}  {:>5.1}%  {:+.0}", e, r * 100.0, elo(r));
    }
}

/// Does deep ISMCTS actually beat the greedy rollout policy? Maps the top span.
fn top_sweep(n: usize) {
    let greedy = Policy::Heuristic { epsilon: 0.0, params: strong_rollout_params() };
    println!("ISMCTS (temp=0) vs GREEDY-heuristic, {} games each:\n", n);
    println!("{:>12}  rate    Elo over greedy", "iters");
    for &iters in &[256u32, 512, 1024, 2048, 4096, 8192] {
        let r = win_rate_pol(&ismcts(iters, 0.0), &greedy, n, 8000 + iters as u64);
        println!("{:>12}  {:>5.1}%  {:+.0}", iters, r * 100.0, elo(r));
    }
}

fn matrix(n: usize) {
    println!("ISMCTS ladder — win rate of HIGHER vs LOWER (target > {:.1}%), {} games each\n", TARGET * 100.0, n);
    println!("{:>12}  {:>10}  rate    margin  pass", "higher", "lower");
    let (mut ok, mut total) = (0, 0);
    let start = Instant::now();
    // Triangular pairing over the ladder — indices are needed for the (i, j>i) pairs.
    #[allow(clippy::needless_range_loop)]
    for i in 0..LADDER.len() {
        for j in (i + 1)..LADDER.len() {
            let lo = LADDER[i];
            let hi = LADDER[j];
            let r = win_rate(hi, lo, n);
            let pass = r > TARGET;
            if pass {
                ok += 1;
            }
            total += 1;
            println!(
                "{:>12}  {:>10}  {:>5.1}%  {:+.1}%  {}",
                name(hi), name(lo), r * 100.0, (r - TARGET) * 100.0,
                if pass { "OK" } else { "FAIL <<<" }
            );
        }
    }
    println!("\nGlobal ordering: {}/{} = {:.1}%   ({:.1}s)", ok, total, ok as f64 / total as f64 * 100.0, start.elapsed().as_secs_f64());
}

/// Measure how many ISMCTS iterations the VPS completes in 200ms.
/// Runs `runs` timed calls of 10,000 iterations each, reports averages.
fn bench(runs: usize) {
    use std::time::Instant;
    let mut rng = StdRng::seed_from_u64(0xBEEFCAFE);
    let state = whots_server::game::engine::create_game(
        vec![seat(Difficulty::TeeNoble), seat(Difficulty::Pikin)],
        whots_server::game::types::GameMode::Stack,
    );
    const ITERS: u32 = 10_000;
    let policy = Policy::Ismcts {
        budget: Budget::Iterations(ITERS),
        temperature: 0.0,
        exploration: 1.4,
        rollout: strong_rollout_params(),
        endgame_samples: 0,
    };

    println!("Timing {ITERS} ISMCTS iterations × {runs} runs on this hardware...\n");
    let mut times_ms = Vec::with_capacity(runs);
    for _ in 0..runs {
        let t = Instant::now();
        let _ = act(&state, 0, &policy, &mut rng);
        times_ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = times_ms.iter().sum::<f64>() / runs as f64;
    let median = times_ms[runs / 2];
    let p95 = times_ms[(runs as f64 * 0.95) as usize];

    let iters_in_200ms = (200.0 / mean * ITERS as f64) as u64;
    println!("  mean:   {mean:.1}ms per {ITERS} iters");
    println!("  median: {median:.1}ms");
    println!("  p95:    {p95:.1}ms");
    println!();
    println!("=> ~{iters_in_200ms} iterations fit in 200ms");
    println!("=> calibrated at 2048 iters; production is {:.1}× that",
        iters_in_200ms as f64 / 2048.0);
    if iters_in_200ms < 2048 {
        println!("WARNING: production is WEAKER than calibration — consider reducing TEE_NOBLE_CAL_ITERS or increasing TEE_NOBLE_PROD_MS");
    }
}

fn curve(n: usize) {
    // Win rate of each level vs pikin (the floor) — shows the usable Elo span.
    println!("Strength vs pikin (floor), {} games each:\n", n);
    println!("{:>12}  rate    Elo gap", "level");
    for &d in &LADDER[1..] {
        let r = win_rate(d, Difficulty::Pikin, n);
        println!("{:>12}  {:>5.1}%  {:+.0}", name(d), r * 100.0, elo(r));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("matrix");
    match cmd {
        "matrix" => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
            matrix(n);
        }
        "curve" => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
            curve(n);
        }
        "grid" => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
            grid(n);
        }
        "eps" => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
            eps_sweep(n);
        }
        "top" => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(500);
            top_sweep(n);
        }
        "pair" => {
            let hi = args.get(2).and_then(|s| parse(s)).expect("usage: pair <hi> <lo> <n>");
            let lo = args.get(3).and_then(|s| parse(s)).expect("usage: pair <hi> <lo> <n>");
            let n = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2000);
            let r = win_rate(hi, lo, n);
            println!("{} vs {}: {:.1}%  (Elo {:+.0})  over {} games", name(hi), name(lo), r * 100.0, elo(r), n);
        }
        "bench" => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
            bench(n);
        }
        _ => eprintln!("unknown command: {cmd}"),
    }
}
