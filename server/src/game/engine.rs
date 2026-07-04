use rand::seq::SliceRandom;
use rand::Rng;
use uuid::Uuid;

use crate::game::{
    deck::shuffled_deck,
    effects::{suit_card_effect, ActionEffect},
    moves::can_play,
    types::{
        Action, Card, GameMode, GamePhase, GameState, GameStateView, PendingEffect, Seat, SeatKind,
        SeatView, Shape, TopCard, SUIT_VALUES,
    },
};

const INITIAL_HAND_SIZE: usize = 5;

#[derive(Debug, thiserror::Error)]
pub enum GameError {
    #[error("game is not in progress")]
    GameNotPlaying,
    #[error("not this player's turn")]
    NotYourTurn,
    #[error("card not in hand")]
    CardNotInHand,
    #[error("invalid move")]
    InvalidMove,
    #[error("player has valid moves and must play a card")]
    MustPlay,
    #[error("player owes a market draw and must draw before playing")]
    MustDraw,
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn advance(state: &GameState, steps: usize) -> usize {
    (state.current_seat_index + steps) % state.seats.len()
}

fn reshuffle_discard(state: &mut GameState) {
    if !state.stock_pile.is_empty() {
        return;
    }
    if state.discard_pile.len() <= 1 {
        return;
    }
    // Keep the top card (most recently added = last element)
    let keep = state.discard_pile.pop().unwrap();
    let mut to_shuffle = std::mem::take(&mut state.discard_pile);
    to_shuffle.shuffle(&mut rand::thread_rng());
    state.stock_pile = to_shuffle;
    state.discard_pile = vec![keep];
}

/// How many cards a single pending penalty card makes you draw: a 2 = 2 cards,
/// a 5 = 3 cards. So a `Pick { count, card }` costs `count * per_card_draw(card)`.
fn per_card_draw(card: u8) -> u32 {
    match card {
        5 => 3,
        _ => 2,
    }
}

/// Draw `count` cards from stock into a seat's hand, reshuffling the discard
/// back into stock as needed. Stops early only if both piles are exhausted.
fn draw_cards(state: &mut GameState, seat_index: usize, count: usize) {
    for _ in 0..count {
        reshuffle_discard(state);
        match state.stock_pile.pop() {
            Some(card) => state.seats[seat_index].hand.push(card),
            None => break,
        }
    }
}

// ── Public API ─────────────────────────────────────────────────────────────────

pub fn create_game(seats: Vec<Seat>, mode: GameMode) -> GameState {
    let mut rng = rand::thread_rng();
    let mut deck = shuffled_deck(&mut rng);

    let n = seats.len();
    let deal_count = (INITIAL_HAND_SIZE * n).min(deck.len());
    let dealt: Vec<Card> = deck.drain(..deal_count).collect();

    let seats_with_hands: Vec<Seat> = seats
        .into_iter()
        .enumerate()
        .map(|(pi, mut seat)| {
            seat.hand = dealt
                .iter()
                .copied()
                .enumerate()
                .filter(|(ci, _)| ci % n == pi)
                .map(|(_, c)| c)
                .collect();
            seat
        })
        .collect();

    // First suit card in remaining deck becomes the starting top card
    let first_suit = deck
        .iter()
        .position(|c| matches!(c, Card::Suit { .. }))
        .expect("deck must contain suit cards");
    let Card::Suit { shape, value } = deck.remove(first_suit) else {
        unreachable!()
    };
    let top_card = TopCard::Suit { shape, value };

    // The starting player is random — nobody is perpetually "seat 0".
    let start = rng.gen_range(0..n);

    let mut state = GameState {
        id: Uuid::new_v4(),
        mode,
        seats: seats_with_hands,
        stock_pile: deck,
        discard_pile: vec![Card::Suit { shape, value }],
        top_card,
        current_seat_index: start,
        phase: GamePhase::Playing,
        pending_effect: None,
        winner_index: None,
    };
    apply_opening_effect(&mut state);
    state
}

/// If the game opens on an action card, its effect lands on the starting player
/// exactly as if an invisible player had just played it:
/// - Pick 2 / Pick 3 (2/5): the starter is under the penalty — counter it (stack
///   mode) or go to market, then play moves on.
/// - Suspension (8): the starter loses their turn; play moves on.
/// - General Market (14): every seat owes a self-draw — the round of draws
///   resolves before play returns to the starter.
/// - Hold On (1) / plain cards: the starter just plays a normal first turn.
/// (Whot never opens — the starting top is always a suit card.)
fn apply_opening_effect(state: &mut GameState) {
    let TopCard::Suit { value, .. } = state.top_card else {
        return;
    };
    let n = state.seats.len();
    match value {
        2 => state.pending_effect = Some(PendingEffect::Pick { count: 1, card: 2 }),
        5 => state.pending_effect = Some(PendingEffect::Pick { count: 1, card: 5 }),
        8 => state.current_seat_index = (state.current_seat_index + 1) % n,
        14 => {
            for s in state.seats.iter_mut() {
                s.owed_draws = 1;
            }
        }
        _ => {}
    }
}

/// Play one or more cards of the **same number** in one turn. A single card is
/// legal in either mode; playing 2+ at once (a stack) is a stack-mode feature.
/// `shapes` lists the cards (one entry per card); all share `value`. Playing m
/// penalty cards against a pending penalty of the same number CANCELS m of them
/// (see `resolve_play`). Otherwise: m 8s = skip m, m 14s = each other owes m
/// self-draws, m 1s = play again, m 2s/5s = start a penalty of that many.
pub fn apply_stack(
    state: &mut GameState,
    seat_index: usize,
    value: u8,
    shapes: &[Shape],
) -> Result<(), GameError> {
    if state.phase != GamePhase::Playing {
        return Err(GameError::GameNotPlaying);
    }
    if state.current_seat_index != seat_index {
        return Err(GameError::NotYourTurn);
    }
    // A General Market obligation must be settled first: you draw your owed
    // cards yourself before you may play anything.
    if state.seats[seat_index].owed_draws > 0 {
        return Err(GameError::MustDraw);
    }
    // Stacking multiple cards is a stack-mode feature. (A single card is fine in
    // either mode; single plays route here with a one-entry `shapes`.)
    if shapes.len() != 1 && state.mode != GameMode::Stack {
        return Err(GameError::InvalidMove);
    }
    if shapes.is_empty() {
        return Err(GameError::InvalidMove);
    }

    // Every listed card must be in hand (respecting duplicates).
    let cards: Vec<Card> = shapes
        .iter()
        .map(|&shape| Card::Suit { shape, value })
        .collect();
    let mut check = state.seats[seat_index].hand.clone();
    for c in &cards {
        match check.iter().position(|x| x == c) {
            Some(pos) => {
                check.remove(pos);
            }
            None => return Err(GameError::CardNotInHand),
        }
    }

    // At least one card must be legal on the pile (this also enforces the
    // number-locked counter when a penalty is pending — all cards share `value`).
    if !cards
        .iter()
        .any(|&c| can_play(c, state.top_card, &state.pending_effect, state.mode))
    {
        return Err(GameError::InvalidMove);
    }

    // Remove the cards and lay them on the pile; the last becomes the top.
    for c in &cards {
        let pos = state.seats[seat_index]
            .hand
            .iter()
            .position(|x| x == c)
            .expect("verified present above");
        state.seats[seat_index].hand.remove(pos);
        state.discard_pile.push(*c);
    }
    let last = *cards.last().expect("non-empty");
    if let Card::Suit { shape, value } = last {
        state.top_card = TopCard::Suit { shape, value };
    }

    let n = cards.len() as u32;
    let effect = suit_card_effect(value);

    let won = state.seats[seat_index].hand.is_empty();
    if won {
        state.phase = GamePhase::Finished;
        state.winner_index = Some(seat_index);
        return Ok(());
    }

    resolve_play(state, value, n, effect);
    Ok(())
}

/// Apply the pending-effect and turn-advance consequences of playing `n` cards
/// of `value` (with card-effect `effect`). Called after the cards are on the
/// pile and a win has been ruled out.
fn resolve_play(state: &mut GameState, value: u8, n: u32, effect: Option<ActionEffect>) {
    // Is this a counter to a pending penalty of the *same* number? (can_play
    // guarantees that if a Pick is pending, the played value matches its card.)
    let pending_count = match &state.pending_effect {
        Some(PendingEffect::Pick { count, card }) if *card == value => Some(*count),
        _ => None,
    };

    match effect {
        Some(ActionEffect::PickTwo) | Some(ActionEffect::PickThree) => {
            if let Some(owed) = pending_count {
                // Counter: cancel card-for-card against the incoming penalty.
                if n >= owed {
                    let excess = n - owed;
                    state.pending_effect = (excess > 0).then_some(PendingEffect::Pick {
                        count: excess,
                        card: value,
                    });
                    // Equal cancels to nothing; excess passes to the next player.
                    state.current_seat_index = advance(state, 1);
                } else {
                    // Under-counter: you cancelled `n`, but still owe `owed - n`.
                    // It stays your turn — you must now draw the remainder (or, if
                    // you somehow hold more, counter again). No advance.
                    state.pending_effect = Some(PendingEffect::Pick {
                        count: owed - n,
                        card: value,
                    });
                }
            } else {
                // Fresh penalty for the next player.
                state.pending_effect = Some(PendingEffect::Pick { count: n, card: value });
                state.current_seat_index = advance(state, 1);
            }
        }
        Some(ActionEffect::HoldOn) => {
            // Play again — keep the seat. Stackable (another 1 replays).
            state.pending_effect = None;
        }
        Some(ActionEffect::Suspension) => {
            state.pending_effect = None;
            state.current_seat_index = advance(state, 1 + n as usize);
        }
        Some(ActionEffect::GeneralMarket) => {
            // Every *other* player owes n self-draws, settled on their own turn.
            // We never draw for them; the game waits until they go to market.
            state.pending_effect = None;
            let current = state.current_seat_index;
            for i in 0..state.seats.len() {
                if i != current {
                    state.seats[i].owed_draws += n;
                }
            }
            state.current_seat_index = advance(state, 1);
        }
        Some(ActionEffect::Whot { .. }) | None => {
            state.pending_effect = None;
            state.current_seat_index = advance(state, 1);
        }
    }
}

fn apply_whot_card(
    state: &mut GameState,
    seat_index: usize,
    called_shape: Shape,
) -> Result<(), GameError> {
    // Settle any General Market obligation before playing.
    if state.seats[seat_index].owed_draws > 0 {
        return Err(GameError::MustDraw);
    }

    let pos = state.seats[seat_index]
        .hand
        .iter()
        .position(|c| matches!(c, Card::Whot))
        .ok_or(GameError::CardNotInHand)?;

    if !can_play(
        Card::Whot,
        state.top_card,
        &state.pending_effect,
        state.mode,
    ) {
        return Err(GameError::InvalidMove);
    }

    state.seats[seat_index].hand.remove(pos);
    let won = state.seats[seat_index].hand.is_empty();
    state.discard_pile.push(Card::Whot);
    state.top_card = TopCard::Whot { called_shape };
    state.pending_effect = None;

    if won {
        state.phase = GamePhase::Finished;
        state.winner_index = Some(seat_index);
        return Ok(());
    }

    // Whot just changes the called shape — no penalty, next player's turn.
    state.current_seat_index = advance(state, 1);
    Ok(())
}

fn apply_draw(state: &mut GameState, seat_index: usize) -> Result<(), GameError> {
    if state.phase != GamePhase::Playing {
        return Err(GameError::GameNotPlaying);
    }
    if state.current_seat_index != seat_index {
        return Err(GameError::NotYourTurn);
    }

    // General Market obligation: take *all* the cards you owe yourself, then your
    // turn ends. This is the "you pick it yourself" flow — the game blocked here
    // waiting for exactly this.
    let owed = state.seats[seat_index].owed_draws;
    if owed > 0 {
        state.seats[seat_index].owed_draws = 0;
        draw_cards(state, seat_index, owed as usize);
        state.current_seat_index = advance(state, 1);
        return Ok(());
    }

    // Pending pick you didn't (fully) counter: draw the owed penalty. `count` is
    // the number of penalty cards; each costs its per-card draw (a 2 = 2, a 5 = 3).
    if let Some(PendingEffect::Pick { count, card }) = state.pending_effect.clone() {
        let draw = (count * per_card_draw(card)) as usize;
        state.pending_effect = None;
        draw_cards(state, seat_index, draw);
        state.current_seat_index = advance(state, 1);
        return Ok(());
    }

    // Voluntary draw: a player may always go to market and take a single card,
    // even when they hold a playable card. Drawing ends the turn.
    draw_cards(state, seat_index, 1);
    state.pending_effect = None;
    state.current_seat_index = advance(state, 1);
    Ok(())
}

pub fn apply_action(
    state: &mut GameState,
    seat_index: usize,
    action: Action,
) -> Result<(), GameError> {
    if state.phase != GamePhase::Playing {
        return Err(GameError::GameNotPlaying);
    }
    if state.current_seat_index != seat_index {
        return Err(GameError::NotYourTurn);
    }
    match action {
        // A single suit card is just a one-card stack.
        Action::PlaySuit { shape, value } => apply_stack(state, seat_index, value, &[shape]),
        Action::PlayWhot { called_shape } => apply_whot_card(state, seat_index, called_shape),
        Action::Draw => apply_draw(state, seat_index),
    }
}

/// Build a player-specific view of the game: the viewer sees their own hand in full;
/// all other seats have an empty hand with only hand_size reflecting truth.
pub fn make_view(state: &GameState, viewer_user_id: Option<Uuid>) -> GameStateView {
    let viewer_seat = viewer_user_id.and_then(|uid| {
        state
            .seats
            .iter()
            .position(|s| matches!(&s.kind, SeatKind::Human { user_id } if *user_id == uid))
    });

    GameStateView {
        id: state.id,
        mode: state.mode,
        seats: state
            .seats
            .iter()
            .enumerate()
            .map(|(i, seat)| SeatView {
                name: seat.name.clone(),
                kind: seat.kind.clone(),
                hand: if Some(i) == viewer_seat {
                    seat.hand.clone()
                } else {
                    vec![]
                },
                hand_size: seat.hand.len(),
                owed_draws: seat.owed_draws,
            })
            .collect(),
        stock_size: state.stock_pile.len(),
        discard_top: state.top_card,
        current_seat_index: state.current_seat_index,
        phase: state.phase,
        pending_effect: state.pending_effect.clone(),
        winner_index: state.winner_index,
    }
}

pub fn is_action_card(value: u8) -> bool {
    matches!(value, 1 | 2 | 5 | 8 | 14)
}

/// Number of suit cards per shape in a full deck (used by AI modules)
pub const CARDS_PER_SHAPE: u32 = SUIT_VALUES.len() as u32;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::types::{Seat, SeatKind};

    fn suit(shape: Shape, value: u8) -> Card {
        Card::Suit { shape, value }
    }
    fn seat(name: &str, hand: Vec<Card>) -> Seat {
        Seat {
            name: name.into(),
            kind: SeatKind::Human {
                user_id: uuid::Uuid::new_v4(),
            },
            hand,
            owed_draws: 0,
        }
    }
    fn state(mode: GameMode, seats: Vec<Seat>, top: (Shape, u8)) -> GameState {
        GameState {
            id: uuid::Uuid::new_v4(),
            mode,
            seats,
            stock_pile: vec![suit(Shape::Circle, 3); 30],
            discard_pile: vec![suit(top.0, top.1)],
            top_card: TopCard::Suit {
                shape: top.0,
                value: top.1,
            },
            current_seat_index: 0,
            phase: GamePhase::Playing,
            pending_effect: None,
            winner_index: None,
        }
    }

    #[test]
    fn stacking_two_twos_piles_penalty() {
        let mut st = state(
            GameMode::Stack,
            vec![
                seat(
                    "A",
                    vec![
                        suit(Shape::Triangle, 2),
                        suit(Shape::Star, 2),
                        suit(Shape::Circle, 7),
                    ],
                ),
                seat("B", vec![suit(Shape::Cross, 10)]),
            ],
            (Shape::Circle, 2),
        );
        apply_stack(&mut st, 0, 2, &[Shape::Triangle, Shape::Star]).unwrap();
        // Two 2s = a penalty of two cards (drawn as 2*2 = 4).
        assert_eq!(
            st.pending_effect,
            Some(PendingEffect::Pick { count: 2, card: 2 })
        );
        assert_eq!(st.current_seat_index, 1);
        assert_eq!(st.seats[0].hand.len(), 1);
    }

    #[test]
    fn no_stack_rejects_multicard() {
        let mut st = state(
            GameMode::NoStack,
            vec![
                seat("A", vec![suit(Shape::Triangle, 2), suit(Shape::Star, 2)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
            ],
            (Shape::Circle, 2),
        );
        assert!(apply_stack(&mut st, 0, 2, &[Shape::Triangle, Shape::Star]).is_err());
    }

    #[test]
    fn counter_is_number_locked() {
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Triangle, 5)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
            ],
            (Shape::Circle, 2),
        );
        st.pending_effect = Some(PendingEffect::Pick { count: 1, card: 2 });
        // A 5 cannot counter a 2-penalty.
        assert!(apply_stack(&mut st, 0, 5, &[Shape::Triangle]).is_err());
    }

    #[test]
    fn equal_counter_cancels_penalty() {
        // Pending 1x2 on A; A answers with one 2 -> clears, advances to B.
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Triangle, 2), suit(Shape::Circle, 9)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
            ],
            (Shape::Circle, 2),
        );
        st.pending_effect = Some(PendingEffect::Pick { count: 1, card: 2 });
        apply_stack(&mut st, 0, 2, &[Shape::Triangle]).unwrap();
        assert_eq!(st.pending_effect, None);
        assert_eq!(st.current_seat_index, 1);
    }

    #[test]
    fn over_counter_passes_excess() {
        // A plays 1x2, B answers with 2x2 -> net one 2 owed by the next player (A).
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Triangle, 2), suit(Shape::Circle, 9)]),
                seat("B", vec![suit(Shape::Star, 2), suit(Shape::Cross, 2), suit(Shape::Square, 4)]),
            ],
            (Shape::Circle, 2),
        );
        apply_stack(&mut st, 0, 2, &[Shape::Triangle]).unwrap();
        assert_eq!(st.pending_effect, Some(PendingEffect::Pick { count: 1, card: 2 }));
        assert_eq!(st.current_seat_index, 1);
        apply_stack(&mut st, 1, 2, &[Shape::Star, Shape::Cross]).unwrap();
        assert_eq!(st.pending_effect, Some(PendingEffect::Pick { count: 1, card: 2 }));
        assert_eq!(st.current_seat_index, 0); // excess lands back on A
    }

    #[test]
    fn under_counter_stays_and_draws_remainder() {
        // Pending 2x5 on A; A plays one 5 -> cancels one, still owes one, stays A's
        // turn. A then draws the remaining 5 (3 cards) and the turn advances.
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Triangle, 5), suit(Shape::Circle, 9)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
            ],
            (Shape::Circle, 5),
        );
        st.pending_effect = Some(PendingEffect::Pick { count: 2, card: 5 });
        apply_stack(&mut st, 0, 5, &[Shape::Triangle]).unwrap();
        assert_eq!(st.pending_effect, Some(PendingEffect::Pick { count: 1, card: 5 }));
        assert_eq!(st.current_seat_index, 0); // still A's turn — must draw remainder
        let before = st.seats[0].hand.len();
        apply_action(&mut st, 0, Action::Draw).unwrap();
        assert_eq!(st.seats[0].hand.len(), before + 3); // one 5 = 3 cards
        assert_eq!(st.pending_effect, None);
        assert_eq!(st.current_seat_index, 1);
    }

    #[test]
    fn general_market_owes_each_other_a_self_draw() {
        // A plays 14 in a 3-player game. B and C each owe one draw; turn advances
        // to B, whose only legal action is to draw (playing a card is rejected).
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Triangle, 14), suit(Shape::Circle, 9)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
                seat("C", vec![suit(Shape::Square, 12)]),
            ],
            (Shape::Circle, 14),
        );
        apply_stack(&mut st, 0, 14, &[Shape::Triangle]).unwrap();
        assert_eq!(st.seats[0].owed_draws, 0);
        assert_eq!(st.seats[1].owed_draws, 1);
        assert_eq!(st.seats[2].owed_draws, 1);
        assert_eq!(st.current_seat_index, 1); // advanced to B, not A "play again"
        // B cannot play while owing a draw.
        assert!(matches!(
            apply_action(&mut st, 1, Action::PlaySuit { shape: Shape::Cross, value: 10 }),
            Err(GameError::MustDraw)
        ));
        // B draws the owed card themselves; turn moves to C.
        let b_before = st.seats[1].hand.len();
        apply_action(&mut st, 1, Action::Draw).unwrap();
        assert_eq!(st.seats[1].hand.len(), b_before + 1);
        assert_eq!(st.seats[1].owed_draws, 0);
        assert_eq!(st.current_seat_index, 2);
    }

    #[test]
    fn opening_pick_penalizes_the_starter() {
        // Game opens on a 2 -> the starting player is under a Pick of one 2.
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Circle, 7)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
            ],
            (Shape::Circle, 2),
        );
        apply_opening_effect(&mut st);
        assert_eq!(st.pending_effect, Some(PendingEffect::Pick { count: 1, card: 2 }));
        assert_eq!(st.current_seat_index, 0);
    }

    #[test]
    fn opening_suspension_skips_the_starter() {
        // Opens on an 8 -> the starting player is suspended, play moves on.
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Circle, 7)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
                seat("C", vec![suit(Shape::Star, 3)]),
            ],
            (Shape::Circle, 8),
        );
        apply_opening_effect(&mut st); // starter = seat 0
        assert_eq!(st.current_seat_index, 1);
        assert_eq!(st.pending_effect, None);
    }

    #[test]
    fn opening_general_market_owes_everyone() {
        // Opens on a 14 -> every seat owes one self-draw before play resumes.
        let mut st = state(
            GameMode::Stack,
            vec![
                seat("A", vec![suit(Shape::Circle, 7)]),
                seat("B", vec![suit(Shape::Cross, 10)]),
                seat("C", vec![suit(Shape::Star, 3)]),
            ],
            (Shape::Circle, 14),
        );
        apply_opening_effect(&mut st);
        assert!(st.seats.iter().all(|s| s.owed_draws == 1));
        assert_eq!(st.current_seat_index, 0); // draws round back to the starter
    }

    #[test]
    fn double_suspension_skips_two() {
        let mut st = state(
            GameMode::Stack,
            vec![
                seat(
                    "A",
                    vec![
                        suit(Shape::Triangle, 8),
                        suit(Shape::Star, 8),
                        suit(Shape::Circle, 3),
                    ],
                ),
                seat("B", vec![suit(Shape::Cross, 10)]),
                seat("C", vec![suit(Shape::Circle, 11)]),
                seat("D", vec![suit(Shape::Square, 12)]),
            ],
            (Shape::Circle, 8),
        );
        apply_stack(&mut st, 0, 8, &[Shape::Triangle, Shape::Star]).unwrap();
        // skip 2 players: from 0, advance 1 + 2 = 3 -> seat 3 (D)
        assert_eq!(st.current_seat_index, 3);
    }
}
