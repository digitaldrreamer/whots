use crate::game::{
    ai::types::{Candidate, ModuleContext},
    types::SHAPES,
};

/// Prefer plays that leave the next player with fewer valid responses.
pub fn anticipation(candidate: &Candidate, ctx: &ModuleContext<'_>) -> f64 {
    let Candidate::PlaySuit { shape, .. } = candidate else {
        return 0.0;
    };

    let shape_remaining = *ctx.shape_remaining.get(shape).unwrap_or(&0) as f64;
    let total_remaining: f64 = SHAPES
        .iter()
        .map(|s| *ctx.shape_remaining.get(s).unwrap_or(&0) as f64)
        .sum();

    if total_remaining == 0.0 {
        return 0.0;
    }

    let n = ctx.state.seats.len();
    let next_idx = (ctx.seat_index + 1) % n;
    let next_hand_size = ctx
        .opponent_hand_sizes
        .get(next_idx)
        .copied()
        .unwrap_or(5)
        .max(0) as f64;

    if next_hand_size == 0.0 {
        return 0.0;
    }

    let p_per_card = shape_remaining / total_remaining;
    let p_can_match = 1.0 - (1.0 - p_per_card).powf(next_hand_size);

    let scale = 1.0 / (n as f64 - 1.0).max(1.0);
    (1.0 - p_can_match) * 8.0 * scale
}
