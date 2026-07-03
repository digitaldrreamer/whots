use crate::game::types::{GameState, Shape};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Candidate {
    PlaySuit { shape: Shape, value: u8 },
    PlayWhot { called_shape: Shape },
    Draw,
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
