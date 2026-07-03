use crate::game::{
    ai::types::{Candidate, ModuleContext},
    engine::CARDS_PER_SHAPE,
};

/// Prefer landing on a shape that has fewer cards remaining — harder for opponents to match.
pub fn card_probability(candidate: &Candidate, ctx: &ModuleContext<'_>) -> f64 {
    let Candidate::PlaySuit { shape, .. } = candidate else {
        return 0.0;
    };

    let remaining = *ctx.shape_remaining.get(shape).unwrap_or(&0) as f64;
    let scale = 1.0 / (ctx.state.seats.len() as f64 - 1.0).max(1.0);
    ((CARDS_PER_SHAPE as f64 - remaining) / CARDS_PER_SHAPE as f64) * 2.0 * scale
}
