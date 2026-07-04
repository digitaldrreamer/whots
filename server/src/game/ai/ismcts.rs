//! Information-Set Monte Carlo Tree Search for Whots.
//!
//! Whots is an imperfect-information, stochastic game: a player cannot see
//! opponents' hands nor the order of the stock pile. ISMCTS handles this by
//! *determinization* — each search iteration samples one concrete world that is
//! consistent with everything the acting seat can observe (its own hand, the
//! full discard pile, every opponent's hand *size*, and the stock *size*), then
//! treats that world as a perfect-information game it can roll out with the real
//! engine.
//!
//! We use single-observer ISMCTS: the tree branches **only on the acting seat's
//! own decisions**. Opponents and chance (draws, reshuffles) are simulated by a
//! fast heuristic policy during both tree descent and rollout. This is faster
//! than a full multi-observer tree, avoids the optimistic-opponent pathology of
//! naive MCTS, and — because real opponents here are weak bots and humans rather
//! than perfect adversaries — actually models the field more accurately.
//!
//! Difficulty is one monotone knob: search budget (iterations or wall-clock)
//! plus a root-selection temperature. More search ⇒ strictly stronger play,
//! which is what makes the ladder cleanly orderable.

use std::time::{Duration, Instant};

use rand::seq::SliceRandom;
use rand::Rng;

use crate::game::{
    ai::{
        context::build_candidates,
        params::{select_move_with_params, select_worst_move_with_params, DifficultyParams},
        types::{AiMove, Candidate},
    },
    deck::create_deck,
    engine::{apply_action, apply_stack, GameError},
    types::{Action, Card, Difficulty, GamePhase, GameState, Shape},
};

/// Resolve an [`AiMove`] against the game state. A `Stack` plays every
/// same-number card of `value` in the acting hand (shapes read from the hand).
pub fn apply_ai_move(state: &mut GameState, seat: usize, mv: AiMove) -> Result<(), GameError> {
    match mv {
        AiMove::Act(action) => apply_action(state, seat, action),
        AiMove::Stack { value } => {
            let shapes: Vec<Shape> = state.seats[seat]
                .hand
                .iter()
                .filter_map(|c| match c {
                    Card::Suit { shape, value: v } if *v == value => Some(*shape),
                    _ => None,
                })
                .collect();
            apply_stack(state, seat, value, &shapes)
        }
    }
}

/// Hard cap on plies simulated in a single rollout, guards against pathological
/// non-terminating games (everyone drawing forever).
const ROLLOUT_PLY_CAP: usize = 400;
/// Cap on consecutive opponent plies between two of our decisions.
const OPP_PLY_CAP: usize = 400;
/// When our hand is this small, the endgame solver replaces ISMCTS rollouts
/// with exhaustive per-action evaluation across many determinizations.
const ENDGAME_THRESHOLD: usize = 3;

// ── Configuration ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum Budget {
    Iterations(u32),
    TimeMs(u64),
}

#[derive(Debug, Clone)]
pub enum Policy {
    /// Uniform random legal move.
    Random,
    /// Deliberately weak "beginner": plays the heuristic's *worst* move with
    /// probability `1 - epsilon`, random otherwise. Lowers the floor so the
    /// bottom rungs of the ladder are clearly separated.
    AntiHeuristic {
        epsilon: f64,
        params: DifficultyParams,
    },
    /// Greedy heuristic with `epsilon` chance of a random move (no search).
    Heuristic {
        epsilon: f64,
        params: DifficultyParams,
    },
    /// Information-set MCTS.
    Ismcts {
        budget: Budget,
        /// Root move selection: 0.0 = pick most-visited (strongest);
        /// higher = sample ∝ visits^(1/temp) (weaker, more varied).
        temperature: f64,
        /// UCB1 exploration constant.
        exploration: f64,
        /// Heuristic weights used for opponents + rollouts inside the search.
        rollout: DifficultyParams,
        /// When > 0 and hand ≤ ENDGAME_THRESHOLD, bypass ISMCTS and evaluate
        /// each legal action across this many random determinizations exactly.
        /// Gives TeeNoble decisive endgame play that ISMCTS rollouts miss.
        endgame_samples: u32,
    },
}

/// A solid heuristic used to drive opponents and rollouts inside the search.
/// All signals on, low noise — realistic play makes the search meaningful.
pub fn strong_rollout_params() -> DifficultyParams {
    DifficultyParams {
        hand_thinning: 1.0,
        action_awareness: 1.0,
        threat_detection: 1.0,
        card_probability: 1.0,
        whot_intelligence: 1.0,
        setup_plays: 1.0,
        anticipation: 1.0,
        noise: 0.05,
        bluff_rate: 0.0,
    }
}

/// A weak heuristic for the lower rungs (basic hand-shedding only).
pub fn weak_params() -> DifficultyParams {
    DifficultyParams {
        hand_thinning: 1.0,
        action_awareness: 0.0,
        threat_detection: 0.0,
        card_probability: 0.0,
        whot_intelligence: 0.0,
        setup_plays: 0.0,
        anticipation: 0.0,
        noise: 0.3,
        bluff_rate: 0.0,
    }
}

/// The difficulty ladder. Strength rises monotonically with search budget;
/// temperature softens the lower rungs so they make beatable mistakes, while a
/// deliberately-bad Pikin floor widens the bottom of the range.
///
/// Budgets are iteration-based so the ladder is reproducible and calibratable.
/// In production, TeeNoble is swapped to a wall-clock budget (see `select_move`)
/// — 200ms buys far more than its calibration iteration count, so it can only be
/// stronger, never weaker, than the calibrated rung.
const TEE_NOBLE_CAL_ITERS: u32 = 2048;
const TEE_NOBLE_PROD_MS: u64 = 200;

pub fn policy_for(difficulty: Difficulty) -> Policy {
    let strong = strong_rollout_params();
    // Top-weighted ladder: the bottom rungs sit close together (gentle
    // onboarding) and the gaps widen toward the top, so climbing toward
    // TeeNoble feels like hitting escalating walls. Approx Elo vs a random
    // floor (measured): pikin 0, smallz +17, isabi +37, chief +58, egbon +119,
    // jagaban +163, tee-noble +213. Bottom four use epsilon-greedy (random →
    // greedy span); top three add ISMCTS search depth on top of greedy.
    match difficulty {
        Difficulty::Pikin => Policy::Random, //   0
        Difficulty::Smallz => Policy::Heuristic {
            epsilon: 0.55,
            params: strong,
        }, // +17
        Difficulty::IsabiSmall => Policy::Heuristic {
            epsilon: 0.42,
            params: strong,
        }, // +37
        Difficulty::Chief => Policy::Heuristic {
            epsilon: 0.25,
            params: strong,
        }, // +58
        Difficulty::Egbon => Policy::Heuristic {
            epsilon: 0.0,
            params: strong,
        }, // +119 (greedy)
        Difficulty::Jagaban => Policy::Ismcts {
            budget: Budget::Iterations(512), // +163
            temperature: 0.0,
            exploration: 1.4,
            rollout: strong,
            endgame_samples: 0,
        },
        Difficulty::TeeNoble => Policy::Ismcts {
            budget: Budget::Iterations(TEE_NOBLE_CAL_ITERS), // +213
            temperature: 0.0,
            exploration: 1.4,
            rollout: strong,
            endgame_samples: 200,
        },
    }
}

// ── Public entry points ──────────────────────────────────────────────────────

/// Select a move for `seat_index` at the given difficulty (production entry).
/// TeeNoble runs under a wall-clock budget here so live latency is bounded
/// regardless of hardware; all other rungs use their calibrated iteration count.
pub fn select_move(state: &GameState, seat_index: usize, difficulty: Difficulty) -> AiMove {
    let mut rng = rand::thread_rng();
    let mut policy = policy_for(difficulty);
    if difficulty == Difficulty::TeeNoble {
        if let Policy::Ismcts { budget, .. } = &mut policy {
            *budget = Budget::TimeMs(TEE_NOBLE_PROD_MS);
        }
    }
    act(state, seat_index, &policy, &mut rng)
}

/// Select a move under an explicit policy. Generic over RNG for seedable tests
/// and calibration.
pub fn act<R: Rng>(state: &GameState, seat_index: usize, policy: &Policy, rng: &mut R) -> AiMove {
    let candidates = build_candidates(state, seat_index);
    // Forced move — no decision to make, skip all machinery.
    if candidates.len() == 1 {
        return candidate_to_aimove(candidates[0]);
    }

    match policy {
        Policy::Random => candidate_to_aimove(*candidates.choose(rng).unwrap()),
        Policy::AntiHeuristic { epsilon, params } => {
            if rng.gen::<f64>() < *epsilon {
                candidate_to_aimove(*candidates.choose(rng).unwrap())
            } else {
                select_worst_move_with_params(state, seat_index, params)
            }
        }
        Policy::Heuristic { epsilon, params } => {
            if rng.gen::<f64>() < *epsilon {
                candidate_to_aimove(*candidates.choose(rng).unwrap())
            } else {
                select_move_with_params(state, seat_index, params)
            }
        }
        Policy::Ismcts {
            budget,
            temperature,
            exploration,
            rollout,
            endgame_samples,
        } => {
            if *endgame_samples > 0 && state.seats[seat_index].hand.len() <= ENDGAME_THRESHOLD {
                if let Some(action) =
                    endgame_solve(state, seat_index, *endgame_samples as usize, rollout, rng)
                {
                    return action;
                }
            }
            ismcts_search(
                state,
                seat_index,
                *budget,
                *temperature,
                *exploration,
                rollout,
                rng,
            )
        }
    }
}

// ── Determinization ──────────────────────────────────────────────────────────

/// Sample one full game state consistent with what `our_seat` can observe:
/// our own hand and the discard pile are fixed; the remaining (unknown) cards —
/// `full_deck − our_hand − discard` — are reshuffled and dealt to the other
/// seats by their known hand sizes, the remainder forming the stock.
pub fn determinize<R: Rng>(state: &GameState, our_seat: usize, rng: &mut R) -> GameState {
    let mut pool = create_deck();
    let mut remove = |c: Card| {
        if let Some(p) = pool.iter().position(|&x| x == c) {
            pool.swap_remove(p);
        }
    };
    for &c in &state.seats[our_seat].hand {
        remove(c);
    }
    for &c in &state.discard_pile {
        remove(c);
    }
    pool.shuffle(rng);

    let mut world = state.clone();
    let mut idx = 0;
    for (i, seat) in world.seats.iter_mut().enumerate() {
        if i == our_seat {
            continue;
        }
        let size = seat.hand.len();
        seat.hand = pool[idx..idx + size].to_vec();
        idx += size;
    }
    world.stock_pile = pool[idx..].to_vec();
    world
}

// ── Tree ─────────────────────────────────────────────────────────────────────

struct Edge {
    action: AiMove,
    child: Option<usize>,
    visits: u32,
    reward: f64,
    avail: u32, // times this action was legal while at the parent node
}

struct Node {
    edges: Vec<Edge>,
}

fn ismcts_search<R: Rng>(
    root_state: &GameState,
    our_seat: usize,
    budget: Budget,
    temperature: f64,
    exploration: f64,
    rollout: &DifficultyParams,
    rng: &mut R,
) -> AiMove {
    let mut arena: Vec<Node> = vec![Node { edges: vec![] }];
    const ROOT: usize = 0;

    let deadline = match budget {
        Budget::TimeMs(ms) => Some(Instant::now() + Duration::from_millis(ms)),
        Budget::Iterations(_) => None,
    };
    let max_iters = match budget {
        Budget::Iterations(n) => n,
        Budget::TimeMs(_) => u32::MAX,
    };

    let mut iters: u32 = 0;
    while iters < max_iters {
        if let Some(dl) = deadline {
            // Check the clock periodically rather than every iteration.
            if iters.is_multiple_of(32) && Instant::now() >= dl {
                break;
            }
        }
        iters += 1;

        let mut state = determinize(root_state, our_seat, rng);
        let mut path: Vec<(usize, usize)> = Vec::new();
        let mut node = ROOT;
        let reward;

        loop {
            // Advance opponents / effects until it is our turn again.
            simulate_until_our_turn(&mut state, our_seat, rollout, rng);
            if state.phase == GamePhase::Finished {
                reward = terminal_reward(&state, our_seat);
                break;
            }
            if state.current_seat_index != our_seat {
                // Opponent loop hit its cap without resolving — bail neutral.
                reward = 0.5;
                break;
            }

            let actions = legal_actions(&state, our_seat);
            if actions.is_empty() {
                reward = 0.5;
                break;
            }

            // Ensure an edge exists for every legal action; bump availability.
            {
                let n = &mut arena[node];
                for a in &actions {
                    if !n.edges.iter().any(|e| e.action == *a) {
                        n.edges.push(Edge {
                            action: *a,
                            child: None,
                            visits: 0,
                            reward: 0.0,
                            avail: 0,
                        });
                    }
                }
                for e in n.edges.iter_mut() {
                    if actions.contains(&e.action) {
                        e.avail += 1;
                    }
                }
            }

            // Expand an untried legal action if one exists.
            let untried: Vec<usize> = arena[node]
                .edges
                .iter()
                .enumerate()
                .filter(|(_, e)| e.child.is_none() && actions.contains(&e.action))
                .map(|(i, _)| i)
                .collect();

            if !untried.is_empty() {
                let pick = untried[rng.gen_range(0..untried.len())];
                let action = arena[node].edges[pick].action;
                let _ = apply_ai_move(&mut state, our_seat, action);
                let child = arena.len();
                arena.push(Node { edges: vec![] });
                arena[node].edges[pick].child = Some(child);
                path.push((node, pick));
                reward = rollout_to_end(&mut state, our_seat, rollout, rng);
                break;
            }

            // Otherwise descend via UCB1 over the currently-legal edges.
            let mut best = f64::NEG_INFINITY;
            let mut best_i = usize::MAX;
            for (i, e) in arena[node].edges.iter().enumerate() {
                if !actions.contains(&e.action) {
                    continue;
                }
                let exploit = e.reward / e.visits as f64;
                let explore = exploration * ((e.avail as f64).ln() / e.visits as f64).sqrt();
                let u = exploit + explore;
                if u > best {
                    best = u;
                    best_i = i;
                }
            }
            let action = arena[node].edges[best_i].action;
            let _ = apply_ai_move(&mut state, our_seat, action);
            path.push((node, best_i));
            node = arena[node].edges[best_i].child.unwrap();
        }

        for (ni, ei) in path {
            let e = &mut arena[ni].edges[ei];
            e.visits += 1;
            e.reward += reward;
        }
    }

    choose_root_action(&arena[ROOT], temperature, rng)
}

// ── Simulation helpers ───────────────────────────────────────────────────────

fn simulate_until_our_turn<R: Rng>(
    state: &mut GameState,
    our_seat: usize,
    params: &DifficultyParams,
    _rng: &mut R,
) {
    let mut steps = 0;
    while state.phase == GamePhase::Playing
        && state.current_seat_index != our_seat
        && steps < OPP_PLY_CAP
    {
        steps += 1;
        let idx = state.current_seat_index;
        let action = select_move_with_params(state, idx, params);
        if apply_ai_move(state, idx, action).is_err() {
            break;
        }
    }
}

fn rollout_to_end<R: Rng>(
    state: &mut GameState,
    our_seat: usize,
    params: &DifficultyParams,
    _rng: &mut R,
) -> f64 {
    let mut plies = 0;
    while state.phase == GamePhase::Playing && plies < ROLLOUT_PLY_CAP {
        plies += 1;
        let idx = state.current_seat_index;
        let action = select_move_with_params(state, idx, params);
        if apply_ai_move(state, idx, action).is_err() {
            break;
        }
    }
    terminal_reward(state, our_seat)
}

fn terminal_reward(state: &GameState, our_seat: usize) -> f64 {
    match state.winner_index {
        Some(w) if w == our_seat => 1.0,
        Some(_) => 0.0,
        None => 0.5,
    }
}

fn legal_actions(state: &GameState, seat_index: usize) -> Vec<AiMove> {
    build_candidates(state, seat_index)
        .into_iter()
        .map(candidate_to_aimove)
        .collect()
}

/// Flat-MC endgame solver. Evaluates each legal action independently across
/// `samples` random determinizations and returns the one that wins most games.
/// More accurate than ISMCTS rollouts when the hand is nearly empty because
/// the game tree is shallow and exhaustive per-action evaluation dominates
/// the noisy UCB statistics built on random rollouts.
fn endgame_solve<R: Rng>(
    state: &GameState,
    our_seat: usize,
    samples: usize,
    rollout_params: &DifficultyParams,
    rng: &mut R,
) -> Option<AiMove> {
    let actions = legal_actions(state, our_seat);
    if actions.len() <= 1 {
        return None; // forced or trivial — let normal path handle it
    }
    let mut wins = vec![0u32; actions.len()];
    for _ in 0..samples {
        let world = determinize(state, our_seat, rng);
        for (i, &action) in actions.iter().enumerate() {
            let mut sim = world.clone();
            if apply_ai_move(&mut sim, our_seat, action).is_ok()
                && rollout_to_end(&mut sim, our_seat, rollout_params, rng) > 0.5
            {
                wins[i] += 1;
            }
        }
    }
    actions
        .into_iter()
        .zip(wins)
        .max_by_key(|(_, w)| *w)
        .map(|(a, _)| a)
}

fn choose_root_action<R: Rng>(root: &Node, temperature: f64, rng: &mut R) -> AiMove {
    let visited: Vec<&Edge> = root.edges.iter().filter(|e| e.visits > 0).collect();
    if visited.is_empty() {
        // No search happened (degenerate); fall back to any known edge.
        return root.edges.first().map(|e| e.action).unwrap_or(AiMove::Act(Action::Draw));
    }

    if temperature <= 0.0 {
        // Strongest: most-visited, tie-broken by mean reward.
        return visited
            .iter()
            .max_by(|a, b| {
                a.visits.cmp(&b.visits).then(
                    (a.reward / a.visits as f64)
                        .partial_cmp(&(b.reward / b.visits as f64))
                        .unwrap(),
                )
            })
            .map(|e| e.action)
            .unwrap();
    }

    // Sample ∝ visits^(1/temperature).
    let inv = 1.0 / temperature;
    let weights: Vec<f64> = visited
        .iter()
        .map(|e| (e.visits as f64).powf(inv))
        .collect();
    let total: f64 = weights.iter().sum();
    let mut r = rng.gen::<f64>() * total;
    for (e, w) in visited.iter().zip(&weights) {
        r -= w;
        if r <= 0.0 {
            return e.action;
        }
    }
    visited.last().unwrap().action
}

fn candidate_to_aimove(c: Candidate) -> AiMove {
    match c {
        Candidate::Draw => AiMove::Act(Action::Draw),
        Candidate::PlaySuit { shape, value } => AiMove::Act(Action::PlaySuit { shape, value }),
        Candidate::PlayWhot { called_shape } => AiMove::Act(Action::PlayWhot { called_shape }),
        Candidate::PlayGroup { value, .. } => AiMove::Stack { value },
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::engine::create_game;
    use crate::game::types::{GameMode, Seat, SeatKind};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn ai_seat(name: &str, d: Difficulty) -> Seat {
        Seat {
            name: name.into(),
            kind: SeatKind::Ai { difficulty: d },
            hand: vec![],
        }
    }

    #[test]
    fn determinize_conserves_the_deck_and_our_hand() {
        let mut rng = StdRng::seed_from_u64(1);
        let state = create_game(
            vec![
                ai_seat("a", Difficulty::TeeNoble),
                ai_seat("b", Difficulty::Pikin),
            ],
            GameMode::Stack,
        );
        let world = determinize(&state, 0, &mut rng);
        // Our hand is untouched.
        assert_eq!(world.seats[0].hand, state.seats[0].hand);
        // Opponent hand sizes preserved.
        assert_eq!(world.seats[1].hand.len(), state.seats[1].hand.len());
        // Total cards conserved (65-card deck).
        let total: usize = world.seats.iter().map(|s| s.hand.len()).sum::<usize>()
            + world.stock_pile.len()
            + world.discard_pile.len();
        assert_eq!(total, create_deck().len());
    }

    #[test]
    fn ismcts_returns_a_legal_move() {
        let mut rng = StdRng::seed_from_u64(7);
        let state = create_game(
            vec![
                ai_seat("a", Difficulty::TeeNoble),
                ai_seat("b", Difficulty::Pikin),
            ],
            GameMode::Stack,
        );
        let policy = Policy::Ismcts {
            budget: Budget::Iterations(50),
            temperature: 0.0,
            exploration: 1.4,
            rollout: strong_rollout_params(),
            endgame_samples: 0,
        };
        let action = act(&state, 0, &policy, &mut rng);
        let legal = legal_actions(&state, 0);
        assert!(
            legal.contains(&action),
            "ISMCTS returned an illegal move: {action:?}"
        );
    }
}
