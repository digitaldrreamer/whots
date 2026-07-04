//! Broad search for AI "identities" — behavior-weight vectors that play with a
//! distinct *style* while staying iso-strength with the balanced brain.
//!
//! An identity = 7 module weights + an application probability `p` (each turn,
//! with prob p it plays its biased pick, else the balanced pick — so a pattern
//! is expressed *sometimes*, never predictably-always). For each candidate we
//! measure win-rate vs the balanced brain (quality) and a behavioral fingerprint
//! (style), then print the iso-strength, mutually-distinct set.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use whots_server::game::{
    ai::{
        context::{build_candidates, build_context},
        ismcts::apply_ai_move,
        modules::{
            action_awareness, anticipation, card_probability, hand_thinning, setup_plays,
            threat_detection, whot_intelligence,
        },
        types::{AiMove, Candidate, ModuleContext},
    },
    engine::create_game,
    types::{Action, Card, Difficulty, GameMode, GamePhase, GameState, PendingEffect, Seat, SeatKind},
};

type W = [f64; 7];
const BALANCED: W = [1.0; 7];
const WNAMES: [&str; 7] = [
    "handThin", "action", "threat", "cardProb", "whotIntel", "setup", "anticip",
];

fn score(c: &Candidate, ctx: &ModuleContext<'_>, w: &W) -> f64 {
    hand_thinning(c, ctx) * w[0]
        + action_awareness(c, ctx) * w[1]
        + threat_detection(c, ctx) * w[2]
        + card_probability(c, ctx) * w[3]
        + whot_intelligence(c, ctx) * w[4]
        + setup_plays(c, ctx) * w[5]
        + anticipation(c, ctx) * w[6]
}

fn cand_to_move(c: Candidate) -> AiMove {
    match c {
        Candidate::Draw => AiMove::Act(Action::Draw),
        Candidate::PlaySuit { shape, value } => AiMove::Act(Action::PlaySuit { shape, value }),
        Candidate::PlayWhot { called_shape } => AiMove::Act(Action::PlayWhot { called_shape }),
        Candidate::PlayGroup { value, .. } => AiMove::Stack { value },
    }
}

fn best(state: &GameState, seat: usize, w: &W, rng: &mut StdRng) -> Candidate {
    let cands = build_candidates(state, seat);
    if cands.len() == 1 {
        return cands[0];
    }
    let ctx = build_context(state, seat, cands.clone());
    let mut best_c = cands[0];
    let mut best_s = f64::NEG_INFINITY;
    for c in &cands {
        let s = score(c, &ctx, w) + rng.gen::<f64>() * 1e-6;
        if s > best_s {
            best_s = s;
            best_c = *c;
        }
    }
    best_c
}

#[derive(Default, Clone)]
struct Fp {
    moves: f64,
    action: f64,
    stack: f64,
    stack_size: f64,
    whot_dump: f64, // played whot when other legal moves existed
    whot_chance: f64,
    hit: f64,     // turns arriving under a pending pick
    countered: f64,
}
impl Fp {
    fn add(&mut self, o: &Fp) {
        self.moves += o.moves;
        self.action += o.action;
        self.stack += o.stack;
        self.stack_size += o.stack_size;
        self.whot_dump += o.whot_dump;
        self.whot_chance += o.whot_chance;
        self.hit += o.hit;
        self.countered += o.countered;
    }
    // Normalised style vector.
    fn vec(&self) -> [f64; 4] {
        let m = self.moves.max(1.0);
        [
            self.action / m,                                  // action-card rate
            self.stack / m,                                   // stacking rate
            if self.whot_chance > 0.0 { self.whot_dump / self.whot_chance } else { 0.0 }, // whot aggression
            if self.hit > 0.0 { self.countered / self.hit } else { 0.0 },                 // counter rate
        ]
    }
}

fn play(w: &W, p: f64, id_seat: usize, seed: u64) -> (Option<bool>, Fp) {
    let mut rng = StdRng::seed_from_u64(seed);
    let seats = vec![
        Seat { name: "0".into(), kind: SeatKind::Ai { difficulty: Difficulty::Chief }, hand: vec![] },
        Seat { name: "1".into(), kind: SeatKind::Ai { difficulty: Difficulty::Chief }, hand: vec![] },
    ];
    let mut state = create_game(seats, GameMode::Stack);
    let mut fp = Fp::default();
    let mut turns = 0;
    while state.phase == GamePhase::Playing && turns < 600 {
        turns += 1;
        let idx = state.current_seat_index;
        let is_id = idx == id_seat;
        let weights = if is_id && rng.gen::<f64>() < p { w } else { &BALANCED };
        // Fingerprint the identity player's actual behaviour.
        if is_id {
            let cands = build_candidates(&state, idx);
            let under_pick = matches!(state.pending_effect, Some(PendingEffect::Pick { .. }));
            let has_whot = state.seats[idx].hand.iter().any(|c| matches!(c, Card::Whot));
            let non_whot_options = cands.iter().any(|c| matches!(c, Candidate::PlaySuit { .. } | Candidate::PlayGroup { .. }));
            let c = best(&state, idx, weights, &mut rng);
            fp.moves += 1.0;
            match c {
                Candidate::PlaySuit { value, .. } if matches!(value, 1 | 2 | 5 | 8 | 14) => fp.action += 1.0,
                Candidate::PlayGroup { value, count, .. } => {
                    fp.stack += 1.0;
                    fp.stack_size += count as f64;
                    if matches!(value, 1 | 2 | 5 | 8 | 14) {
                        fp.action += 1.0;
                    }
                }
                _ => {}
            }
            if has_whot && non_whot_options {
                fp.whot_chance += 1.0;
                if matches!(c, Candidate::PlayWhot { .. }) {
                    fp.whot_dump += 1.0;
                }
            }
            if under_pick {
                fp.hit += 1.0;
                if !matches!(c, Candidate::Draw) {
                    fp.countered += 1.0;
                }
            }
            if apply_ai_move(&mut state, idx, cand_to_move(c)).is_err() {
                break;
            }
        } else {
            let c = best(&state, idx, weights, &mut rng);
            if apply_ai_move(&mut state, idx, cand_to_move(c)).is_err() {
                break;
            }
        }
    }
    (state.winner_index.map(|win| win == id_seat), fp)
}

fn eval(w: &W, p: f64, n: usize, salt: u64) -> (f64, [f64; 4]) {
    let results: Vec<(u32, u32, Fp)> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (won, fp) = play(w, p, i % 2, salt.wrapping_add(i as u64));
            match won {
                Some(true) => (1, 1, fp),
                Some(false) => (0, 1, fp),
                None => (0, 0, fp),
            }
        })
        .collect();
    let mut wins = 0u32;
    let mut total = 0u32;
    let mut fp = Fp::default();
    for (w_, t, f) in &results {
        wins += w_;
        total += t;
        fp.add(f);
    }
    let wr = if total == 0 { 0.5 } else { wins as f64 / total as f64 };
    (wr, fp.vec())
}

fn dist(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
}

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(120);
    let samples: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(400);

    // Baseline fingerprint of the pure balanced brain (for distinctiveness).
    let (_, base_fp) = eval(&BALANCED, 0.0, n, 1);

    // Build candidate weight vectors: 7 "pure" (emphasise one module), plus random.
    let mut cands: Vec<W> = Vec::new();
    for i in 0..7 {
        let mut w = BALANCED;
        w[i] = 3.0;
        cands.push(w);
        let mut w2 = BALANCED;
        w2[i] = 0.0;
        cands.push(w2);
    }
    let mut rng = StdRng::seed_from_u64(42);
    for _ in 0..samples {
        let mut w = BALANCED;
        for x in w.iter_mut() {
            *x = rng.gen::<f64>() * 3.0;
        }
        cands.push(w);
    }

    // Evaluate each at three application probabilities.
    let ps = [0.6f64, 0.8, 1.0];
    let mut rows: Vec<(W, f64, f64, [f64; 4], f64)> = Vec::new(); // w, p, winrate, fp, style-dist-from-balanced
    for (ci, w) in cands.iter().enumerate() {
        for &p in &ps {
            let (wr, fp) = eval(w, p, n, (ci as u64) * 31 + (p * 100.0) as u64);
            let d = dist(&fp, &base_fp);
            rows.push((*w, p, wr, fp, d));
        }
    }

    // Iso-strength survivors, most distinct first.
    let mut iso: Vec<&(W, f64, f64, [f64; 4], f64)> = rows.iter().filter(|r| r.2 >= 0.49).collect();
    iso.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap());

    println!("\n=== Balanced fingerprint [action, stack, whotAgg, counter]: {:?} ===", base_fp.map(|x| (x * 100.0).round() / 100.0));
    println!("=== {} candidates x {} p-values = {} evals; {} iso-strength (>=49%) ===\n", cands.len(), ps.len(), rows.len(), iso.len());

    // Greedily pick a diverse set (max-min style distance), each iso-strength.
    let mut chosen: Vec<&(W, f64, f64, [f64; 4], f64)> = Vec::new();
    for cand in &iso {
        if chosen.len() >= 8 {
            break;
        }
        let far_enough = chosen.iter().all(|c| dist(&c.3, &cand.3) > 0.06);
        if far_enough && cand.4 > 0.03 {
            chosen.push(cand);
        }
    }

    println!("── Diverse iso-strength identities (win% vs balanced, p, style fingerprint) ──\n");
    println!("  {:<7} {:>4} | {:<38} | {:<28}", "win%", "p", "top weights", "style [act stk whot ctr]");
    for r in &chosen {
        let (w, p, wr, fp, _) = r;
        // top 2 emphasised weights
        let mut idx: Vec<usize> = (0..7).collect();
        idx.sort_by(|&a, &b| w[b].partial_cmp(&w[a]).unwrap());
        let top = format!("{} {:.1}, {} {:.1}, {} {:.1}", WNAMES[idx[0]], w[idx[0]], WNAMES[idx[1]], w[idx[1]], WNAMES[idx[2]], w[idx[2]]);
        println!(
            "  {:<7.1} {:>4.1} | {:<38} | [{:.2} {:.2} {:.2} {:.2}]",
            wr * 100.0, p, top, fp[0], fp[1], fp[2], fp[3]
        );
    }
    println!("\n  style axes: act=action-card rate, stk=stacking rate, whot=whot-aggression, ctr=counter rate");
}
