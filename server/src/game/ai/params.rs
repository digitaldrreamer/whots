use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::game::{
    ai::{
        context::{build_candidates, build_context},
        modules::{
            action_awareness, anticipation, card_probability, hand_thinning, setup_plays,
            threat_detection, whot_intelligence,
        },
        types::{Candidate, ModuleContext},
    },
    types::{Action, Difficulty, GameState},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyParams {
    pub hand_thinning:     f64,
    pub action_awareness:  f64,
    pub threat_detection:  f64,
    pub card_probability:  f64,
    pub whot_intelligence: f64,
    pub setup_plays:       f64,
    pub anticipation:      f64,
    pub noise:             f64,
    pub bluff_rate:        f64,
}

impl DifficultyParams {
    pub fn zero() -> Self {
        Self {
            hand_thinning: 0.0, action_awareness: 0.0, threat_detection: 0.0,
            card_probability: 0.0, whot_intelligence: 0.0, setup_plays: 0.0,
            anticipation: 0.0, noise: 0.0, bluff_rate: 0.0,
        }
    }
}

fn default_params(difficulty: Difficulty) -> DifficultyParams {
    match difficulty {
        Difficulty::Pikin => DifficultyParams { noise: 100.0, ..DifficultyParams::zero() },
        Difficulty::Smallz => DifficultyParams { hand_thinning: 1.0, noise: 0.1, ..DifficultyParams::zero() },
        Difficulty::IsabiSmall => DifficultyParams { hand_thinning: 1.0, action_awareness: 1.0, noise: 0.1, ..DifficultyParams::zero() },
        Difficulty::Chief => DifficultyParams { hand_thinning: 1.0, action_awareness: 1.0, threat_detection: 1.0, noise: 0.1, ..DifficultyParams::zero() },
        Difficulty::Egbon => DifficultyParams {
            hand_thinning: 1.0, action_awareness: 1.0, threat_detection: 1.0,
            card_probability: 1.0, whot_intelligence: 1.0, setup_plays: 1.0,
            noise: 0.1, ..DifficultyParams::zero()
        },
        Difficulty::Jagaban => DifficultyParams {
            hand_thinning: 1.0, action_awareness: 1.0, threat_detection: 1.0,
            card_probability: 1.0, whot_intelligence: 1.0, setup_plays: 1.0,
            anticipation: 1.0, noise: 0.05, ..DifficultyParams::zero()
        },
        Difficulty::TeeNoble => DifficultyParams {
            hand_thinning: 1.0, action_awareness: 1.0, threat_detection: 1.0,
            card_probability: 1.0, whot_intelligence: 1.0, setup_plays: 1.0,
            anticipation: 1.0, noise: 0.02, ..DifficultyParams::zero()
        },
    }
}

fn gaussian_noise(sigma: f64, rng: &mut impl Rng) -> f64 {
    if sigma <= 0.0 { return 0.0; }
    let u1 = rng.gen::<f64>().max(1e-10);
    let u2 = rng.gen::<f64>();
    sigma * (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

fn score(candidate: &Candidate, ctx: &ModuleContext<'_>, p: &DifficultyParams, rng: &mut impl Rng) -> f64 {
    hand_thinning(candidate, ctx)     * p.hand_thinning
    + action_awareness(candidate, ctx)  * p.action_awareness
    + threat_detection(candidate, ctx)  * p.threat_detection
    + card_probability(candidate, ctx)  * p.card_probability
    + whot_intelligence(candidate, ctx) * p.whot_intelligence
    + setup_plays(candidate, ctx)       * p.setup_plays
    + anticipation(candidate, ctx)      * p.anticipation
    + gaussian_noise(p.noise, rng)
}

fn candidate_to_action(c: Candidate) -> Action {
    match c {
        Candidate::Draw                       => Action::Draw,
        Candidate::PlaySuit { shape, value }  => Action::PlaySuit { shape, value },
        Candidate::PlayWhot { called_shape }  => Action::PlayWhot { called_shape },
    }
}

/// Select move using default params for a difficulty level
pub fn select_move(state: &GameState, seat_index: usize, difficulty: Difficulty) -> Action {
    let params = default_params(difficulty);
    select_move_with_params(state, seat_index, &params)
}

/// Select move using custom params
pub fn select_move_with_params(state: &GameState, seat_index: usize, params: &DifficultyParams) -> Action {
    let candidates = build_candidates(state, seat_index);
    if candidates.iter().all(|c| matches!(c, Candidate::Draw)) {
        return Action::Draw;
    }

    let ctx = build_context(state, seat_index, candidates.clone());
    let mut rng = rand::thread_rng();

    let mut scored: Vec<(Candidate, f64)> = candidates
        .iter()
        .map(|c| (*c, score(c, &ctx, params, &mut rng)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let chosen = if params.bluff_rate > 0.0 && scored.len() > 1 && rng.gen::<f64>() < params.bluff_rate {
        scored[1].0
    } else {
        scored[0].0
    };

    candidate_to_action(chosen)
}

/// Select the *worst*-scoring legal move per the heuristic — used to build a
/// deliberately weak "beginner" floor (Pikin) that plays plausibly but badly,
/// which widens the bottom of the difficulty ladder.
pub fn select_worst_move_with_params(state: &GameState, seat_index: usize, params: &DifficultyParams) -> Action {
    let candidates = build_candidates(state, seat_index);
    if candidates.iter().all(|c| matches!(c, Candidate::Draw)) {
        return Action::Draw;
    }

    let ctx = build_context(state, seat_index, candidates.clone());
    let mut rng = rand::thread_rng();

    let worst = candidates
        .iter()
        .map(|c| (*c, score(c, &ctx, params, &mut rng)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(c, _)| c)
        .unwrap_or(Candidate::Draw);

    candidate_to_action(worst)
}

/// Make DifficultyParams public constructors
impl DifficultyParams {
    pub fn clamp(&mut self) {
        macro_rules! clamp {
            ($f:tt) => { self.$f = self.$f.max(0.0).min(4.0) };
        }
        clamp!(hand_thinning);
        clamp!(action_awareness);
        clamp!(threat_detection);
        clamp!(card_probability);
        clamp!(whot_intelligence);
        clamp!(setup_plays);
        clamp!(anticipation);
        self.noise = self.noise.clamp(0.0, 200.0);
        self.bluff_rate = self.bluff_rate.clamp(0.0, 0.5);
    }
}
