use rand::seq::SliceRandom;
use uuid::Uuid;

use crate::game::{
    deck::shuffled_deck,
    effects::{suit_card_effect, ActionEffect},
    moves::{can_play, valid_moves},
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
            let base = if let Some(PendingEffect::Pick { total }) = current {
                total
            } else {
                0
            };
            Some(PendingEffect::Pick { total: base + 2 })
        }
        Some(ActionEffect::PickThree) => {
            let base = if let Some(PendingEffect::Pick { total }) = current {
                total
            } else {
                0
            };
            Some(PendingEffect::Pick { total: base + 3 })
        }
        Some(ActionEffect::HoldOn) => Some(PendingEffect::Skip),
        Some(
            ActionEffect::Suspension | ActionEffect::GeneralMarket | ActionEffect::Whot { .. },
        ) => None,
    }
}

fn resolve_next_turn(state: &mut GameState, effect: Option<ActionEffect>) {
    if state.winner_index.is_some() {
        return;
    }
    match effect {
        Some(ActionEffect::HoldOn) => {
            // A gets a follow-up turn — stay on same seat
            // pending_effect is already Skip (set in apply_suit_card)
        }
        Some(ActionEffect::Suspension) => {
            state.pending_effect = None;
            state.current_seat_index = advance(state, 2);
        }
        Some(ActionEffect::GeneralMarket) => {
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
            state.current_seat_index = advance(state, 1);
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
    if let Some(PendingEffect::Pick { total }) = state.pending_effect.clone() {
        let count = total as usize;
        state.pending_effect = None;
        reshuffle_discard(state);
        let take = count.min(state.stock_pile.len());
        let drawn: Vec<Card> = (0..take).filter_map(|_| state.stock_pile.pop()).collect();
        state.seats[seat_index].hand.extend(drawn);
        state.current_seat_index = advance(state, 1);
        return Ok(());
    }

    // Normal draw: only valid when there are no playable cards
    let valid = valid_moves(
        &state.seats[seat_index].hand,
        state.top_card,
        &state.pending_effect,
        state.mode,
    );
    if !valid.is_empty() {
        return Err(GameError::MustPlay);
    }

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
