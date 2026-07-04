use super::types::{Candidate, ModuleContext};
use crate::game::{
    engine::CARDS_PER_SHAPE,
    moves::valid_moves,
    types::{Card, GameMode, GameState, Shape, SHAPES},
};
use std::collections::HashMap;

pub fn build_candidates(state: &GameState, seat_index: usize) -> Vec<Candidate> {
    let Some(seat) = state.seats.get(seat_index) else {
        return vec![Candidate::Draw];
    };

    let valid = valid_moves(
        &seat.hand,
        state.top_card,
        &state.pending_effect,
        state.mode,
    );
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
                    candidates.push(Candidate::PlayWhot {
                        called_shape: shape,
                    });
                }
                whot_expanded = true;
            }
            Card::Whot => {}
        }
    }

    // Group (stack) moves — stack mode only: for any number you hold 2+ of and
    // can legally lead, offer "play all of them" as one move. A legal lead's
    // shape becomes top_shape (used to score the group as its representative
    // single). Partial stacks are omitted — "one" and "all" cover the strategy.
    if state.mode == GameMode::Stack {
        let mut hand_counts: HashMap<u8, u8> = HashMap::new();
        for card in &seat.hand {
            if let Card::Suit { value, .. } = card {
                *hand_counts.entry(*value).or_insert(0) += 1;
            }
        }
        // Legal lead shape per value (first legal single of that value).
        let mut lead_shape: HashMap<u8, Shape> = HashMap::new();
        for c in &candidates {
            if let Candidate::PlaySuit { shape, value } = c {
                lead_shape.entry(*value).or_insert(*shape);
            }
        }
        let mut groups: Vec<Candidate> = lead_shape
            .iter()
            .filter_map(|(&value, &top_shape)| {
                let count = *hand_counts.get(&value).unwrap_or(&0);
                (count >= 2).then_some(Candidate::PlayGroup {
                    value,
                    count,
                    top_shape,
                })
            })
            .collect();
        candidates.append(&mut groups);
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
        .map(|(i, s)| {
            if i == seat_index {
                -1
            } else {
                s.hand.len() as i32
            }
        })
        .collect();

    ModuleContext {
        state,
        seat_index,
        candidates,
        opponent_hand_sizes,
        shape_remaining,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::types::{GameMode, GamePhase, Seat, SeatKind, TopCard};
    use uuid::Uuid;

    #[test]
    fn build_candidates_offers_a_group_for_two_same_number() {
        // Seat 0 holds two 7s (one matches the top by number) -> expect a PlayGroup{7}.
        let seat = Seat {
            name: "a".into(),
            kind: SeatKind::Ai { difficulty: crate::game::types::Difficulty::Chief },
            hand: vec![
                Card::Suit { shape: Shape::Triangle, value: 7 },
                Card::Suit { shape: Shape::Star, value: 7 },
                Card::Suit { shape: Shape::Circle, value: 3 },
            ],
        };
        let st = GameState {
            id: Uuid::new_v4(),
            mode: GameMode::Stack,
            seats: vec![seat, Seat { name: "b".into(), kind: SeatKind::Ai { difficulty: crate::game::types::Difficulty::Pikin }, hand: vec![Card::Suit { shape: Shape::Cross, value: 10 }] }],
            stock_pile: vec![Card::Suit { shape: Shape::Circle, value: 3 }; 20],
            discard_pile: vec![Card::Suit { shape: Shape::Circle, value: 7 }],
            top_card: TopCard::Suit { shape: Shape::Circle, value: 7 },
            current_seat_index: 0,
            phase: GamePhase::Playing,
            pending_effect: None,
            winner_index: None,
        };
        let cands = build_candidates(&st, 0);
        assert!(cands.iter().any(|c| matches!(c, Candidate::PlayGroup { value: 7, count: 2, .. })),
            "expected a PlayGroup{{7}} candidate, got {cands:?}");
    }

    #[test]
    fn no_group_candidates_in_no_stack_mode() {
        let seat = Seat {
            name: "a".into(),
            kind: SeatKind::Ai { difficulty: crate::game::types::Difficulty::Chief },
            hand: vec![
                Card::Suit { shape: Shape::Triangle, value: 7 },
                Card::Suit { shape: Shape::Star, value: 7 },
            ],
        };
        let st = GameState {
            id: Uuid::new_v4(),
            mode: GameMode::NoStack,
            seats: vec![seat, Seat { name: "b".into(), kind: SeatKind::Ai { difficulty: crate::game::types::Difficulty::Pikin }, hand: vec![Card::Suit { shape: Shape::Cross, value: 10 }] }],
            stock_pile: vec![Card::Suit { shape: Shape::Circle, value: 3 }; 20],
            discard_pile: vec![Card::Suit { shape: Shape::Circle, value: 7 }],
            top_card: TopCard::Suit { shape: Shape::Circle, value: 7 },
            current_seat_index: 0,
            phase: GamePhase::Playing,
            pending_effect: None,
            winner_index: None,
        };
        let cands = build_candidates(&st, 0);
        assert!(!cands.iter().any(|c| matches!(c, Candidate::PlayGroup { .. })));
    }
}
