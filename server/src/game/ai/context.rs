use std::collections::HashMap;
use crate::game::{
    engine::CARDS_PER_SHAPE,
    moves::valid_moves,
    types::{Card, GameState, Shape, SHAPES},
};
use super::types::{Candidate, ModuleContext};

pub fn build_candidates(state: &GameState, seat_index: usize) -> Vec<Candidate> {
    let Some(seat) = state.seats.get(seat_index) else {
        return vec![Candidate::Draw];
    };

    let valid = valid_moves(&seat.hand, state.top_card, &state.pending_effect, state.mode);
    if valid.is_empty() {
        return vec![Candidate::Draw];
    }

    let mut candidates = Vec::new();
    let mut whot_expanded = false;

    for card in valid {
        match card {
            Card::Suit { shape, value } => candidates.push(Candidate::PlaySuit { shape, value }),
            Card::Whot if !whot_expanded => {
                for shape in SHAPES {
                    candidates.push(Candidate::PlayWhot { called_shape: shape });
                }
                whot_expanded = true;
            }
            Card::Whot => {}
        }
    }

    candidates
}

pub fn build_context<'a>(
    state: &'a GameState,
    seat_index: usize,
    candidates: Vec<Candidate>,
) -> ModuleContext<'a> {
    let mut accounted: HashMap<Shape, u32> = SHAPES.iter().map(|&s| (s, 0)).collect();

    for card in &state.discard_pile {
        if let Card::Suit { shape, .. } = card {
            *accounted.entry(*shape).or_insert(0) += 1;
        }
    }
    if let Some(seat) = state.seats.get(seat_index) {
        for card in &seat.hand {
            if let Card::Suit { shape, .. } = card {
                *accounted.entry(*shape).or_insert(0) += 1;
            }
        }
    }

    let shape_remaining: HashMap<Shape, u32> = SHAPES
        .iter()
        .map(|&s| {
            let used = *accounted.get(&s).unwrap_or(&0);
            (s, CARDS_PER_SHAPE.saturating_sub(used))
        })
        .collect();

    let opponent_hand_sizes: Vec<i32> = state
        .seats
        .iter()
        .enumerate()
        .map(|(i, s)| if i == seat_index { -1 } else { s.hand.len() as i32 })
        .collect();

    ModuleContext {
        state,
        seat_index,
        candidates,
        opponent_hand_sizes,
        shape_remaining,
    }
}
