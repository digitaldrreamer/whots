use crate::game::{
    ai::types::{Candidate, ModuleContext},
    engine::is_action_card,
    moves::valid_moves,
    types::{Card, TopCard},
};

/// Prefer plays that leave us with more follow-up options next turn.
pub fn setup_plays(candidate: &Candidate, ctx: &ModuleContext<'_>) -> f64 {
    if matches!(candidate, Candidate::Draw) {
        return 0.0;
    }

    let Some(seat) = ctx.state.seats.get(ctx.seat_index) else {
        return 0.0;
    };

    let simulated_top: TopCard = match candidate {
        Candidate::PlaySuit { shape, value } => TopCard::Suit {
            shape: *shape,
            value: *value,
        },
        // Groups are scored via their representative single in params.rs, so this
        // module never actually receives one; treat it as its top card anyway.
        Candidate::PlayGroup { value, top_shape, .. } => TopCard::Suit {
            shape: *top_shape,
            value: *value,
        },
        Candidate::PlayWhot { called_shape } => TopCard::Whot {
            called_shape: *called_shape,
        },
        Candidate::Draw => unreachable!(),
    };

    // Remove exactly one instance of the played card from our hand
    let played_idx = seat.hand.iter().position(|c| match (candidate, c) {
        (
            Candidate::PlaySuit { shape, value },
            Card::Suit {
                shape: cs,
                value: cv,
            },
        ) => cs == shape && cv == value,
        (Candidate::PlayWhot { .. }, Card::Whot) => true,
        _ => false,
    });
    let Some(played_idx) = played_idx else {
        return 0.0;
    };

    let remaining: Vec<Card> = seat
        .hand
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != played_idx)
        .map(|(_, &c)| c)
        .collect();

    // Playing last card — always right
    if remaining.is_empty() {
        return 50.0;
    }

    let follow_ups = valid_moves(&remaining, simulated_top, &None, ctx.state.mode);
    let scale = 1.0 / (ctx.state.seats.len() as f64 - 1.0).max(1.0);

    if follow_ups.is_empty() {
        if let Candidate::PlaySuit { value, .. } = candidate {
            if is_action_card(*value) {
                return 0.0;
            }
        }
        return -20.0 * scale;
    }

    (follow_ups.len() as f64 * 1.5).min(6.0) * scale
}
