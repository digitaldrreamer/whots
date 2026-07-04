use crate::game::{
    ai::types::{Candidate, ModuleContext},
    engine::CARDS_PER_SHAPE,
};

/// When playing a Whot (or declaring an opening Whot's shape), call the shape
/// opponents are least likely to hold.
pub fn whot_intelligence(candidate: &Candidate, ctx: &ModuleContext<'_>) -> f64 {
    let called_shape = match candidate {
        Candidate::PlayWhot { called_shape } => called_shape,
        Candidate::CallShape { shape } => shape,
        _ => return 0.0,
    };

    // For an actual Whot play, don't bias toward it when a suit play exists — let
    // suit cards win. An opening CallShape has no alternative, so always score it.
    if matches!(candidate, Candidate::PlayWhot { .. })
        && ctx
            .candidates
            .iter()
            .any(|c| matches!(c, Candidate::PlaySuit { .. }))
    {
        return 0.0;
    }

    let remaining = *ctx.shape_remaining.get(called_shape).unwrap_or(&0) as f64;
    let scale = 1.0 / (ctx.state.seats.len() as f64 - 1.0).max(1.0);
    ((CARDS_PER_SHAPE as f64 - remaining) / CARDS_PER_SHAPE as f64) * 15.0 * scale
}
