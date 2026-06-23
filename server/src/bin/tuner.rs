/// WHOTS AI Difficulty Tuner — Rust edition
///
/// Coordinate-descent optimizer for DifficultyParams.
/// Goal: find params for each level so every higher level beats every lower level
/// by at least TARGET_MARGIN in both 1v1 and 4-player games.
///
/// Usage:
///   cargo run --release --bin tuner -- --games 500 --continuous
///   cargo run --release --bin tuner -- --verify
///   cargo run --release --bin tuner -- --resume --continuous

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use whots_server::game::{
    ai::params::{DifficultyParams, select_move_with_params},
    engine::{create_game, apply_action},
    types::{Action, Difficulty, GameMode, GameState, Seat, SeatKind, GamePhase},
};

// ── Configuration ──────────────────────────────────────────────────────────────

const GAMES_PER_EVAL: usize = 500;
const MAX_SWEEPS: usize = 5000;
const INITIAL_STEP: f64 = 0.15;
const MIN_STEP: f64 = 0.005;
const STEP_DECAY: f64 = 0.6;
const CHECKPOINT_PATH: &str = "scripts/params/best.json";
const MAX_TURNS: usize = 600;

// Ceiling test: ISMCTS tee-noble wins ~80.7% vs pikin at 200 sims.
// Elo span 248 / 6 steps = 41 Elo/step → 1/(1+10^(-41/400)) = 55.9%.
const TARGET_MARGIN: f64 = 0.559;
const MULTI_WEIGHT: f64 = 0.35;
const MULTI_GAMES_RATIO: f64 = 0.4;
const PERTURB_STRENGTH: f64 = 0.5;

const LADDER: &[Difficulty] = &[
    Difficulty::Pikin,
    Difficulty::Smallz,
    Difficulty::IsabiSmall,
    Difficulty::Chief,
    Difficulty::Egbon,
    Difficulty::Jagaban,
    Difficulty::TeeNoble,
];

const TUNABLE: &[Difficulty] = &[
    Difficulty::Smallz,
    Difficulty::IsabiSmall,
    Difficulty::Chief,
    Difficulty::Egbon,
    Difficulty::Jagaban,
];

const PARAM_NAMES: &[&str] = &[
    "hand_thinning",
    "action_awareness",
    "threat_detection",
    "card_probability",
    "whot_intelligence",
    "setup_plays",
    "anticipation",
    "noise",
    "bluff_rate",
];

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Checkpoint {
    meta: CheckpointMeta,
    params: HashMap<String, DifficultyParams>,
}

#[derive(Serialize, Deserialize)]
struct CheckpointMeta {
    sweeps_completed: usize,
    games_per_eval: usize,
    saved_at: String,
    best_global_score: f64,
}

struct Args {
    games: usize,
    sweeps: usize,
    step: f64,
    resume: bool,
    verify: bool,
    continuous: bool,
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn parse_args() -> Args {
    let mut args = Args {
        games: GAMES_PER_EVAL,
        sweeps: MAX_SWEEPS,
        step: INITIAL_STEP,
        resume: false,
        verify: false,
        continuous: false,
    };

    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--games" if i + 1 < argv.len() => {
                args.games = argv[i + 1].parse().unwrap_or(GAMES_PER_EVAL);
                i += 2;
            }
            "--sweeps" if i + 1 < argv.len() => {
                args.sweeps = argv[i + 1].parse().unwrap_or(MAX_SWEEPS);
                i += 2;
            }
            "--step" if i + 1 < argv.len() => {
                args.step = argv[i + 1].parse().unwrap_or(INITIAL_STEP);
                i += 2;
            }
            "--resume" => {
                args.resume = true;
                i += 1;
            }
            "--verify" => {
                args.verify = true;
                i += 1;
            }
            "--continuous" => {
                args.continuous = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    args
}

fn default_params(difficulty: Difficulty) -> DifficultyParams {
    match difficulty {
        Difficulty::Pikin => DifficultyParams {
            noise: 100.0,
            ..DifficultyParams::zero()
        },
        Difficulty::Smallz => DifficultyParams {
            hand_thinning: 1.0,
            noise: 0.1,
            ..DifficultyParams::zero()
        },
        Difficulty::IsabiSmall => DifficultyParams {
            hand_thinning: 1.15,
            action_awareness: 1.0,
            threat_detection: 0.15,
            noise: 0.1,
            ..DifficultyParams::zero()
        },
        Difficulty::Chief => DifficultyParams {
            hand_thinning: 0.85,
            action_awareness: 1.15,
            threat_detection: 1.0,
            noise: 0.1,
            ..DifficultyParams::zero()
        },
        Difficulty::Egbon => DifficultyParams {
            hand_thinning: 1.0,
            action_awareness: 1.0,
            threat_detection: 0.85,
            card_probability: 1.0,
            whot_intelligence: 1.0,
            setup_plays: 1.0,
            noise: 0.1,
            ..DifficultyParams::zero()
        },
        Difficulty::Jagaban => DifficultyParams {
            hand_thinning: 1.0,
            action_awareness: 1.0,
            threat_detection: 1.0,
            card_probability: 1.0,
            whot_intelligence: 1.0,
            setup_plays: 1.0,
            anticipation: 1.0,
            noise: 0.05,
            ..DifficultyParams::zero()
        },
        Difficulty::TeeNoble => DifficultyParams {
            hand_thinning: 1.0,
            action_awareness: 1.0,
            threat_detection: 1.0,
            card_probability: 1.0,
            whot_intelligence: 1.0,
            setup_plays: 1.0,
            anticipation: 1.0,
            noise: 0.02,
            ..DifficultyParams::zero()
        },
    }
}

fn difficulty_name(d: Difficulty) -> &'static str {
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

// ── Game simulation ────────────────────────────────────────────────────────────

fn simulate_one(
    level_a: Difficulty,
    level_b: Difficulty,
    params_a: &DifficultyParams,
    params_b: &DifficultyParams,
    a_first: bool,
) -> Option<bool> {
    let (first, second, first_params, second_params) = if a_first {
        (level_a, level_b, params_a, params_b)
    } else {
        (level_b, level_a, params_b, params_a)
    };

    let seats = vec![
        Seat { name: difficulty_name(first).to_string(), kind: SeatKind::Ai { difficulty: first }, hand: vec![] },
        Seat { name: difficulty_name(second).to_string(), kind: SeatKind::Ai { difficulty: second }, hand: vec![] },
    ];
    let mut state = create_game(seats, GameMode::Stack);
    let mut turns = 0;

    while state.phase == GamePhase::Playing && turns < MAX_TURNS {
        turns += 1;
        let idx = state.current_seat_index;
        let params = if idx == 0 { first_params } else { second_params };

        let action = select_move_with_params(&state, idx, params);
        if let Err(_) = apply_action(&mut state, idx, action) {
            break;
        }
    }

    if let Some(winner_idx) = state.winner_index {
        let a_won = if a_first { winner_idx == 0 } else { winner_idx == 1 };
        Some(a_won)
    } else {
        None
    }
}

fn win_rate(
    level_a: Difficulty,
    level_b: Difficulty,
    params_a: &DifficultyParams,
    params_b: &DifficultyParams,
    n: usize,
) -> f64 {
    let mut wins = 0;
    let mut total = 0;

    for i in 0..n {
        if let Some(a_won) = simulate_one(level_a, level_b, params_a, params_b, i % 2 == 0) {
            if a_won {
                wins += 1;
            }
            total += 1;
        }
    }

    if total == 0 { 0.5 } else { wins as f64 / total as f64 }
}

// ── Objectives ─────────────────────────────────────────────────────────────────

fn global_ranking_score(
    params: &HashMap<String, DifficultyParams>,
    games_per_matchup: usize,
) -> (f64, Vec<Vec<f64>>) {
    let mut correct = 0;
    let mut total = 0;
    let mut matrix = vec![vec![0.5; LADDER.len()]; LADDER.len()];

    for i in 0..LADDER.len() {
        for j in (i + 1)..LADDER.len() {
            let lo = LADDER[i];
            let hi = LADDER[j];
            let params_lo = &params[difficulty_name(lo)];
            let params_hi = &params[difficulty_name(hi)];
            let rate = win_rate(hi, lo, params_hi, params_lo, games_per_matchup);

            matrix[j][i] = rate;
            matrix[i][j] = 1.0 - rate;

            if rate > TARGET_MARGIN {
                correct += 1;
            }
            total += 1;
        }
    }

    ((correct as f64 / total as f64), matrix)
}

fn quick_objective(
    level: Difficulty,
    params: &HashMap<String, DifficultyParams>,
    games_per_matchup: usize,
) -> f64 {
    let pos = LADDER.iter().position(|&d| d == level).unwrap();
    let mut score = 0.0;
    let mut terms = 0;

    if pos > 0 {
        let below = LADDER[pos - 1];
        let rate = win_rate(level, below, &params[difficulty_name(level)], &params[difficulty_name(below)], games_per_matchup);
        score += rate - TARGET_MARGIN;
        terms += 1;
    }

    if pos < LADDER.len() - 1 {
        let above = LADDER[pos + 1];
        let rate = win_rate(level, above, &params[difficulty_name(level)], &params[difficulty_name(above)], games_per_matchup);
        score -= rate;
        terms += 1;
    }

    if terms > 0 { score / terms as f64 * 2.0 } else { 0.0 }
}

fn load_checkpoint() -> Option<HashMap<String, DifficultyParams>> {
    if !Path::new(CHECKPOINT_PATH).exists() {
        return None;
    }

    if let Ok(json) = fs::read_to_string(CHECKPOINT_PATH) {
        if let Ok(checkpoint) = serde_json::from_str::<Checkpoint>(&json) {
            println!("  Resumed from checkpoint (sweeps: {}, saved: {})", checkpoint.meta.sweeps_completed, checkpoint.meta.saved_at);
            return Some(checkpoint.params);
        }
    }

    println!("  Could not parse checkpoint — starting fresh");
    None
}

fn save_checkpoint(params: &HashMap<String, DifficultyParams>, sweeps: usize, best_score: f64) {
    fs::create_dir_all("scripts/params").ok();
    let checkpoint = Checkpoint {
        meta: CheckpointMeta {
            sweeps_completed: sweeps,
            games_per_eval: GAMES_PER_EVAL,
            saved_at: chrono::Local::now().to_rfc3339(),
            best_global_score: best_score,
        },
        params: params.clone(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&checkpoint) {
        let _ = fs::write(CHECKPOINT_PATH, json);
    }
}

// ── Main ───────────────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  WHOTS AI TUNER (Rust)                                   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    println!("  Games per eval  : {}", args.games);
    let sweep_str = args.sweeps.to_string();
    println!("  Max sweeps      : {}", if args.continuous { "∞ (continuous)".to_string() } else { sweep_str });
    println!("  Initial step    : {}", args.step);
    println!("  Continuous      : {}", if args.continuous { "yes" } else { "no" });
    println!("  Checkpoint      : {}\n", CHECKPOINT_PATH);

    // Load or init params
    let mut params: HashMap<String, DifficultyParams> = if args.resume {
        load_checkpoint().unwrap_or_else(|| {
            LADDER.iter().map(|&d| (difficulty_name(d).to_string(), default_params(d))).collect()
        })
    } else {
        LADDER.iter().map(|&d| (difficulty_name(d).to_string(), default_params(d))).collect()
    };

    if args.verify {
        println!("── Verification run ────────────────────────────────────────\n");
        let (score, _) = global_ranking_score(&params, args.games.max(1000));
        println!("  Ordering score: {:.1}%\n", score * 100.0);
        return;
    }

    println!("── Baseline evaluation ─────────────────────────────────────\n");
    let (base_score, _) = global_ranking_score(&params, args.games);
    println!("  Baseline ordering score: {:.1}%\n", base_score * 100.0);

    let mut best_params = params.clone();
    let mut best_global_score = base_score;
    let mut all_time_best_params = params.clone();
    let mut all_time_best_score = base_score;
    let mut step = args.step;
    let mut total_sweeps = 0;
    let mut restart_count = 0;

    for sweep in 1..=args.sweeps {
        total_sweeps += 1;
        let sweep_label = if args.continuous {
            format!("Sweep {}/{}  restart #{}  total {}", sweep, args.sweeps, restart_count, total_sweeps)
        } else {
            format!("Sweep {}/{}", sweep, args.sweeps)
        };

        println!("\n── {}  (step={:.4}) ─────────────────────────────\n", sweep_label, step);

        let mut improved_this_sweep = false;

        for level in TUNABLE {
            let level_name = difficulty_name(*level);
            let mut best_level_score = quick_objective(*level, &best_params, args.games);
            let mut best_level_params = best_params[level_name].clone();

            for param_idx in 0..PARAM_NAMES.len() {
                for &delta_sign in &[1.0, -1.0] {
                    let delta = step * delta_sign;
                    let mut candidate = best_level_params.clone();

                    let old_val = match param_idx {
                        0 => { candidate.hand_thinning += delta; best_level_params.hand_thinning },
                        1 => { candidate.action_awareness += delta; best_level_params.action_awareness },
                        2 => { candidate.threat_detection += delta; best_level_params.threat_detection },
                        3 => { candidate.card_probability += delta; best_level_params.card_probability },
                        4 => { candidate.whot_intelligence += delta; best_level_params.whot_intelligence },
                        5 => { candidate.setup_plays += delta; best_level_params.setup_plays },
                        6 => { candidate.anticipation += delta; best_level_params.anticipation },
                        7 => { candidate.noise += delta; best_level_params.noise },
                        8 => { candidate.bluff_rate += delta; best_level_params.bluff_rate },
                        _ => 0.0,
                    };

                    candidate.clamp();
                    let mut test_params = best_params.clone();
                    test_params.insert(level_name.to_string(), candidate.clone());
                    let score = quick_objective(*level, &test_params, args.games);

                    if score > best_level_score + 0.001 {
                        let delta_obj = (score - best_level_score) * 100.0;
                        let new_val = match param_idx {
                            0 => candidate.hand_thinning,
                            1 => candidate.action_awareness,
                            2 => candidate.threat_detection,
                            3 => candidate.card_probability,
                            4 => candidate.whot_intelligence,
                            5 => candidate.setup_plays,
                            6 => candidate.anticipation,
                            7 => candidate.noise,
                            8 => candidate.bluff_rate,
                            _ => 0.0,
                        };
                        best_level_score = score;
                        best_level_params = candidate;
                        println!("  {}  {} {:.3} → {:.3}  (Δobj {:.1}%)", level_name, PARAM_NAMES[param_idx], old_val, new_val, delta_obj);
                    }
                }
            }

            if best_level_params != best_params[level_name] {
                best_params.insert(level_name.to_string(), best_level_params);
                let (new_global, _) = global_ranking_score(&best_params, args.games);

                if new_global >= best_global_score {
                    best_global_score = new_global;
                    improved_this_sweep = true;
                    if new_global > all_time_best_score {
                        all_time_best_score = new_global;
                        all_time_best_params = best_params.clone();
                        save_checkpoint(&all_time_best_params, total_sweeps, all_time_best_score);
                        println!("  ✓ {}: new all-time best {:.1}%  [saved]\n", level_name, new_global * 100.0);
                    } else {
                        println!("  ✓ {}: local improvement {:.1}%\n", level_name, new_global * 100.0);
                    }
                } else {
                    best_params.insert(level_name.to_string(), params[level_name].clone());
                    println!("  ✗ {}: reverted (global dropped to {:.1}%)\n", level_name, new_global * 100.0);
                }
            }
        }

        if !improved_this_sweep {
            if step > MIN_STEP {
                step *= STEP_DECAY;
                println!("  No improvement — step → {:.4}", step);
            } else if args.continuous {
                restart_count += 1;
                println!("\n  Converged — restart #{} (perturbing all-time best)\n", restart_count);
                let mut rng = rand::thread_rng();
                let mut perturbed = all_time_best_params.clone();
                for level in TUNABLE.iter() {
                    let level_name = difficulty_name(*level);
                    let p = &mut perturbed.get_mut(level_name).unwrap();
                    p.hand_thinning += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.action_awareness += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.threat_detection += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.card_probability += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.whot_intelligence += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.setup_plays += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.anticipation += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.noise += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.bluff_rate += (rng.gen::<f64>() * 2.0 - 1.0) * PERTURB_STRENGTH;
                    p.clamp();
                }
                params = perturbed.clone();
                best_params = perturbed;
                let (perturbed_baseline, _) = global_ranking_score(&params, args.games);
                best_global_score = perturbed_baseline;
                step = args.step;
            } else {
                println!("\n  Converged (step at minimum, no improvement).");
                break;
            }
        }
    }

    println!("\n\n══ Final results ═══════════════════════════════════════════\n");
    let final_games = (args.games * 4).max(2000);
    println!("Running final verification with {} games per matchup...\n", final_games);
    let (final_score, _) = global_ranking_score(&all_time_best_params, final_games);

    println!("  Final ordering score : {:.1}% pairs correct", final_score * 100.0);
    println!("  All-time best (tuner): {:.1}%", all_time_best_score * 100.0);
    if args.continuous {
        println!("  Total restarts       : {}", restart_count);
    }
    println!("  Total sweeps         : {}", total_sweeps);
    println!("\n  Results saved to     : {}\n", CHECKPOINT_PATH);

    save_checkpoint(&all_time_best_params, total_sweeps, final_score);
}
