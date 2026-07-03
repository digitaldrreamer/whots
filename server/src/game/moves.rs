use crate::game::types::{Card, GameMode, PendingEffect, TopCard};

pub fn can_play(card: Card, top: TopCard, pending: &Option<PendingEffect>, mode: GameMode) -> bool {
    // In stack mode with a pending pick, only 2s and 5s can counter
    if let Some(PendingEffect::Pick { .. }) = pending {
        if mode == GameMode::Stack {
            return matches!(
                card,
                Card::Suit { value: 2, .. } | Card::Suit { value: 5, .. }
            );
        }
    }

    // Whot is always playable outside a counter window
    if matches!(card, Card::Whot) {
        return true;
    }

    let Card::Suit {
        shape: cs,
        value: cv,
    } = card
    else {
        return false;
    };

    match top {
        TopCard::Whot { called_shape } => cs == called_shape,
        TopCard::Suit {
            shape: ts,
            value: tv,
        } => cs == ts || cv == tv,
    }
}

pub fn valid_moves(
    hand: &[Card],
    top: TopCard,
    pending: &Option<PendingEffect>,
    mode: GameMode,
) -> Vec<Card> {
    hand.iter()
        .copied()
        .filter(|&c| can_play(c, top, pending, mode))
        .collect()
}
