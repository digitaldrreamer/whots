use rand::seq::SliceRandom;
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

fn compute_pending(
    effect: Option<ActionEffect>,
    current: Option<PendingEffect>,
) -> Option<PendingEffect> {
    match effect {
        None => current,
        Some(ActionEffect::PickTwo) => {
            // Number-locked: only piles onto an existing 2-chain (can_play blocks
            // playing a 2 onto a 5-penalty in the first place).
            let base = match current {
                Some(PendingEffect::Pick { total, card: 2 }) => total,
                _ => 0,
            };
            Some(PendingEffect::Pick {
                total: base + 2,
                card: 2,
            })
        }
        Some(ActionEffect::PickThree) => {
            let base = match current {
                Some(PendingEffect::Pick { total, card: 5 }) => total,
                _ => 0,
            };
            Some(PendingEffect::Pick {
                total: base + 3,
                card: 5,
            })
        }
        // Hold On and General Market are pure "play again" — no pending effect
        // carried into the follow-up turn.
        Some(
            ActionEffect::HoldOn
            | ActionEffect::Suspension
            | ActionEffect::GeneralMarket
            | ActionEffect::Whot { .. },
        ) => None,
    }
}

fn resolve_next_turn(state: &mut GameState, effect: Option<ActionEffect>) {
    if state.winner_index.is_some() {
        return;
    }
    match effect {
        Some(ActionEffect::HoldOn) => {
            // "Play again" — keep the same seat. Stackable (another 1 replays).
        }
        Some(ActionEffect::Suspension) => {
            state.pending_effect = None;
            state.current_seat_index = advance(state, 2);
        }
        Some(ActionEffect::GeneralMarket) => {
            // Every other player draws one card, then the player plays again
            // (same "play again" flow as Hold On — keep the same seat).
            reshuffle_discard(state);
            let current = state.current_seat_index;
            let n = state.seats.len();
            for i in 0..n {
                if i == current {
                    continue;
                }
                if let Some(card) = state.stock_pile.pop() {
                    state.seats[i].hand.push(card);
                }
            }
            state.pending_effect = None;
        }
        _ => {
            // pick_two, pick_three, whot, non-action, or end of hold_on chain
            let skipping = matches!(state.pending_effect, Some(PendingEffect::Skip));
            if skipping {
                state.pending_effect = None;
            }
            state.current_seat_index = advance(state, if skipping { 2 } else { 1 });
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

    GameState {
        id: Uuid::new_v4(),
        mode,
        seats: seats_with_hands,
        stock_pile: deck,
        discard_pile: vec![Card::Suit { shape, value }],
        top_card,
        current_seat_index: 0,
        phase: GamePhase::Playing,
        pending_effect: None,
        winner_index: None,
    }
}

fn apply_suit_card(
    state: &mut GameState,
    seat_index: usize,
    shape: Shape,
    value: u8,
) -> Result<(), GameError> {
    let card = Card::Suit { shape, value };
    let pos = state.seats[seat_index]
        .hand
        .iter()
        .position(|&c| c == card)
        .ok_or(GameError::CardNotInHand)?;

    if !can_play(card, state.top_card, &state.pending_effect, state.mode) {
        return Err(GameError::InvalidMove);
    }

    let effect = suit_card_effect(value);
    let new_pending = compute_pending(effect, state.pending_effect.clone());

    state.seats[seat_index].hand.remove(pos);
    let won = state.seats[seat_index].hand.is_empty();
    state.discard_pile.push(card);
    state.top_card = TopCard::Suit { shape, value };
    state.pending_effect = new_pending;

    if won {
        state.phase = GamePhase::Finished;
        state.winner_index = Some(seat_index);
    }

    resolve_next_turn(state, effect);
    Ok(())
}

/// Play several cards of the **same number** in one turn (stack mode only).
/// `shapes` lists the cards (one entry per card); all share `value`. Effects
/// accumulate: n 2s = +2n, n 5s = +3n, n 8s = skip n, n 14s = others draw n each,
/// n 1s = play again. A single-card list behaves like `apply_suit_card`.
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
    // Stacking multiple cards is a stack-mode feature. (A single card is fine in
    // either mode, but single plays come through apply_suit_card.)
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
    state.pending_effect = compute_pending_stack(effect, state.pending_effect.clone(), n);

    let won = state.seats[seat_index].hand.is_empty();
    if won {
        state.phase = GamePhase::Finished;
        state.winner_index = Some(seat_index);
    }

    resolve_stack_turn(state, effect, n);
    Ok(())
}

fn compute_pending_stack(
    effect: Option<ActionEffect>,
    current: Option<PendingEffect>,
    n: u32,
) -> Option<PendingEffect> {
    match effect {
        Some(ActionEffect::PickTwo) => {
            let base = match current {
                Some(PendingEffect::Pick { total, card: 2 }) => total,
                _ => 0,
            };
            Some(PendingEffect::Pick {
                total: base + 2 * n,
                card: 2,
            })
        }
        Some(ActionEffect::PickThree) => {
            let base = match current {
                Some(PendingEffect::Pick { total, card: 5 }) => total,
                _ => 0,
            };
            Some(PendingEffect::Pick {
                total: base + 3 * n,
                card: 5,
            })
        }
        _ => None,
    }
}

fn resolve_stack_turn(state: &mut GameState, effect: Option<ActionEffect>, n: u32) {
    if state.winner_index.is_some() {
        return;
    }
    match effect {
        // Play again — keep the seat (stackable via chaining is moot here, the
        // whole same-number group was one play).
        Some(ActionEffect::HoldOn) => {}
        Some(ActionEffect::GeneralMarket) => {
            reshuffle_discard(state);
            let current = state.current_seat_index;
            let seats = state.seats.len();
            for i in 0..seats {
                if i == current {
                    continue;
                }
                for _ in 0..n {
                    if let Some(card) = state.stock_pile.pop() {
                        state.seats[i].hand.push(card);
                    }
                }
            }
            state.pending_effect = None;
        }
        // Skip n players.
        Some(ActionEffect::Suspension) => {
            state.pending_effect = None;
            state.current_seat_index = advance(state, 1 + n as usize);
        }
        // Pick-two / pick-three (penalty passes on) or plain cards: next player.
        _ => {
            state.current_seat_index = advance(state, 1);
        }
    }
}

fn apply_whot_card(
    state: &mut GameState,
    seat_index: usize,
    called_shape: Shape,
) -> Result<(), GameError> {
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
    }

    resolve_next_turn(state, Some(ActionEffect::Whot { called_shape }));
    Ok(())
}

fn apply_draw(state: &mut GameState, seat_index: usize) -> Result<(), GameError> {
    if state.phase != GamePhase::Playing {
        return Err(GameError::GameNotPlaying);
    }
    if state.current_seat_index != seat_index {
        return Err(GameError::NotYourTurn);
    }

    // Pending pick — player couldn't counter; draw the full accumulated total
    if let Some(PendingEffect::Pick { total, .. }) = state.pending_effect.clone() {
        let count = total as usize;
        state.pending_effect = None;
        reshuffle_discard(state);
        let take = count.min(state.stock_pile.len());
        let drawn: Vec<Card> = (0..take).filter_map(|_| state.stock_pile.pop()).collect();
        state.seats[seat_index].hand.extend(drawn);
        state.current_seat_index = advance(state, 1);
        return Ok(());
    }

    // Voluntary draw: a player may always go to market and take a single card,
    // even when they hold a playable card. Drawing ends the turn.
    let skipping = matches!(state.pending_effect, Some(PendingEffect::Skip));
    reshuffle_discard(state);

    if let Some(card) = state.stock_pile.pop() {
        state.seats[seat_index].hand.push(card);
    }
    state.pending_effect = None;
    state.current_seat_index = advance(state, if skipping { 2 } else { 1 });

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
        Action::PlaySuit { shape, value } => apply_suit_card(state, seat_index, shape, value),
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
        assert_eq!(
            st.pending_effect,
            Some(PendingEffect::Pick { total: 4, card: 2 })
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
        st.pending_effect = Some(PendingEffect::Pick { total: 2, card: 2 });
        // A 5 cannot counter a 2-penalty.
        assert!(apply_stack(&mut st, 0, 5, &[Shape::Triangle]).is_err());
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
