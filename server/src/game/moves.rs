use crate::game::types::{Card, GameMode, PendingEffect, TopCard};

pub fn can_play(card: Card, top: TopCard, pending: &Option<PendingEffect>, mode: GameMode) -> bool {
    // An opening Whot must be resolved by declaring a shape (a CallShape action),
    // not by playing a card — nothing is playable until then.
    if matches!(pending, Some(PendingEffect::CallShape)) {
        return false;
    }

    // Under a pending penalty, countering is number-locked: only the same
    // number that started it (a 2-chain answered with 2s, a 5-chain with 5s),
    // and only in stack mode. No-stack has no counter — the player must draw.
    if let Some(PendingEffect::Pick { card: counter, .. }) = pending {
        return mode == GameMode::Stack
            && matches!(card, Card::Suit { value, .. } if value == *counter);
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
