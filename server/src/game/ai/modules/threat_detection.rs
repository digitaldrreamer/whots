use crate::game::ai::types::{Candidate, ModuleContext};

const THREAT_THRESHOLD: i32 = 3;

/// When the next player is close to winning, prefer hindering action cards.
pub fn threat_detection(candidate: &Candidate, ctx: &ModuleContext<'_>) -> f64 {
    let Candidate::PlaySuit { value, .. } = candidate else { return 0.0 };

    let n = ctx.state.seats.len();
    let next_idx = (ctx.seat_index + 1) % n;
    let next_size = ctx.opponent_hand_sizes.get(next_idx).copied().unwrap_or(i32::MAX);
    let min_opp = ctx
        .opponent_hand_sizes
        .iter()
        .filter(|&&s| s != -1)
        .copied()
        .min()
        .unwrap_or(i32::MAX);

    let threat_is_next = next_size <= THREAT_THRESHOLD && next_size == min_opp;

    // Hinder cards aimed at next player
    if threat_is_next && matches!(value, 1 | 2 | 5 | 8) {
        return 20.0;
    }
    // General market hurts everyone — bonus when anyone is close
    if *value == 14 && min_opp <= THREAT_THRESHOLD {
        return 15.0;
    }

    0.0
}
