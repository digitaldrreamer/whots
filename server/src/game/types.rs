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
    // `count` = how many penalty cards are owed; `card` = the number that started
    // it (2 or 5). Countering is number-locked (a 2-chain answered only with 2s)
    // and CANCELS card-for-card: play fewer than owed and you draw the leftover,
    // play more and the excess passes on, play equal and it clears. The actual
    // cards drawn = count * per-card (a 2 = 2 cards, a 5 = 3 cards).
    Pick {
        #[serde(default, alias = "total")]
        count: u32,
        card: u8,
    },
    Skip,
    /// The game opened on a Whot: the current (starting) player must declare a
    /// shape before anyone can play. Their only legal move is CallShape.
    CallShape,
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
    /// Cards this player must draw *themselves* on their turn before doing
    /// anything else — set by another player's General Market. The game waits
    /// on them (a human clicks market; the AI draws automatically); we never
    /// toss the card into their hand for them.
    #[serde(default)]
    pub owed_draws: u32,
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
    pub owed_draws: u32,  // General Market cards this seat still has to draw
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
    /// Declare the shape for an opening Whot (the game started on a Whot). No card
    /// leaves the hand; play then passes to the next player, who must match.
    CallShape { called_shape: Shape },
    Draw,
}
