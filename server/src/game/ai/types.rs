use crate::game::types::{Action, GameState, Shape};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    PlaySuit {
        shape: Shape,
        value: u8,
    },
    /// Play *all* same-number cards of `value` in one turn (stack mode). `count`
    /// is how many will be played; `top_shape` is a legal lead's shape (used to
    /// score the group as its representative single).
    PlayGroup {
        value: u8,
        count: u8,
        top_shape: Shape,
    },
    PlayWhot {
        called_shape: Shape,
    },
    Draw,
}

/// A resolved AI move. `Stack` plays every same-number card of `value` from the
/// acting seat's hand (shapes reconstructed at apply time), keeping this enum
/// `Copy` and cheap inside the search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMove {
    Act(Action),
    Stack { value: u8 },
}

pub struct ModuleContext<'a> {
    pub state: &'a GameState,
    pub seat_index: usize,
    pub candidates: Vec<Candidate>,
    /// Hand size for each seat (-1 for the acting seat)
    pub opponent_hand_sizes: Vec<i32>,
    /// Estimated suit cards of each shape still held by opponents or in stock
    pub shape_remaining: HashMap<Shape, u32>,
}

pub type ScoringFn = fn(&Candidate, &ModuleContext<'_>) -> f64;
