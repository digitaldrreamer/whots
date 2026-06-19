use crate::game::{ai::types::{Candidate, ModuleContext}, engine::CARDS_PER_SHAPE};

/// When forced to play Whot, call the shape opponents are least likely to hold.
pub fn whot_intelligence(candidate: &Candidate, ctx: &ModuleContext<'_>) -> f64 {
    let Candidate::PlayWhot { called_shape } = candidate else { return 0.0 };

    // If any suit play is available, don't bias toward Whot — let suit cards win
    if ctx.candidates.iter().any(|c| matches!(c, Candidate::PlaySuit { .. })) {
        return 0.0;
    }

    let remaining = *ctx.shape_remaining.get(called_shape).unwrap_or(&0) as f64;
    let scale = 1.0 / (ctx.state.seats.len() as f64 - 1.0).max(1.0);
    ((CARDS_PER_SHAPE as f64 - remaining) / CARDS_PER_SHAPE as f64) * 15.0 * scale
}
