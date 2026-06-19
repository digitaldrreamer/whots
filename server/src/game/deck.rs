use rand::seq::SliceRandom;
use crate::game::types::{Card, SHAPES, SUIT_VALUES, WHOT_COUNT};

pub fn create_deck() -> Vec<Card> {
    let mut deck: Vec<Card> = SHAPES
        .iter()
        .flat_map(|&shape| SUIT_VALUES.iter().map(move |&value| Card::Suit { shape, value }))
        .collect();
    for _ in 0..WHOT_COUNT {
        deck.push(Card::Whot);
    }
    deck
}

pub fn shuffled_deck(rng: &mut impl rand::Rng) -> Vec<Card> {
    let mut deck = create_deck();
    deck.shuffle(rng);
    deck
}
