use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const SHAPES: [Shape; 5] = [
    Shape::Circle,
    Shape::Triangle,
    Shape::Cross,
    Shape::Square,
    Shape::Star,
];
pub const SUIT_VALUES: [u8; 12] = [1, 2, 3, 4, 5, 7, 8, 10, 11, 12, 13, 14];
pub const WHOT_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Circle,
    Triangle,
    Cross,
    Square,
    Star,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Card {
    Suit { shape: Shape, value: u8 },
    Whot,
}

/// The effective top of the discard pile used for move validation.
/// A played Whot carries the shape the player called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TopCard {
    Suit { shape: Shape, value: u8 },
    Whot { called_shape: Shape },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    Stack,
    NoStack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Pikin,
    Smallz,
    IsabiSmall,
    Chief,
    Egbon,
    Jagaban,
    TeeNoble,
}

impl Difficulty {
    pub fn to_db_str(self) -> &'static str {
        match self {
            Difficulty::Pikin => "pikin",
            Difficulty::Smallz => "smallz",
            Difficulty::IsabiSmall => "isabi_small",
            Difficulty::Chief => "chief",
            Difficulty::Egbon => "egbon",
            Difficulty::Jagaban => "jagaban",
            Difficulty::TeeNoble => "tee_noble",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePhase {
    Playing,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PendingEffect {
    // `card` is the number that started the penalty (2 or 5). Countering is
    // number-locked: a Pick started by a 2 can only be answered with 2s.
    Pick { total: u32, card: u8 },
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatKind {
    Human { user_id: Uuid },
    Ai { difficulty: Difficulty },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seat {
    pub name: String,
    pub kind: SeatKind,
    pub hand: Vec<Card>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub id: Uuid,
    pub mode: GameMode,
    pub seats: Vec<Seat>,
    pub stock_pile: Vec<Card>,
    pub discard_pile: Vec<Card>,
    pub top_card: TopCard,
    pub current_seat_index: usize,
    pub phase: GamePhase,
    pub pending_effect: Option<PendingEffect>,
    pub winner_index: Option<usize>,
}

/// Per-player view of a seat: own hand visible, opponents' hands hidden.
#[derive(Debug, Clone, Serialize)]
pub struct SeatView {
    pub name: String,
    pub kind: SeatKind,
    pub hand: Vec<Card>,  // populated only for the viewing player
    pub hand_size: usize, // always the true count
}

/// What the server sends to each client — tailored so they see only their own cards.
#[derive(Debug, Clone, Serialize)]
pub struct GameStateView {
    pub id: Uuid,
    pub mode: GameMode,
    pub seats: Vec<SeatView>,
    pub stock_size: usize,
    pub discard_top: TopCard,
    pub current_seat_index: usize,
    pub phase: GamePhase,
    pub pending_effect: Option<PendingEffect>,
    pub winner_index: Option<usize>,
}

/// The action a player (human or AI) takes on their turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    PlaySuit { shape: Shape, value: u8 },
    PlayWhot { called_shape: Shape },
    Draw,
}
