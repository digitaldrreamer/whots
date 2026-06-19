use crate::game::{ai::types::{Candidate, ModuleContext}, types::Card};

/// Prefer playing from the shape we hold the most of.
/// Shedding the dominant shape fastest reduces the chance of getting stuck.
pub fn hand_thinning(candidate: &Candidate, ctx: &ModuleContext<'_>) -> f64 {
    let Candidate::PlaySuit { shape, .. } = candidate else { return 0.0 };
    let Some(seat) = ctx.state.seats.get(ctx.seat_index) else { return 0.0 };

    seat.hand
        .iter()
        .filter(|c| matches!(c, Card::Suit { shape: s, .. } if s == shape))
        .count() as f64
}
