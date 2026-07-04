use std::{collections::HashSet, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    auth::WsAuthUser,
    error::AppError,
    game::{
        ai::{apply_ai_move, select_move, AiMove},
        engine::{apply_action, apply_stack, make_view, GameError},
        types::{Action, GamePhase, GameState, GameStateView, SeatKind, Shape},
    },
    state::AppState,
    store::game_store,
};

// ── Room registry ──────────────────────────────────────────────────────────────

/// A move from a human: either a single card/draw, or a same-number stack.
pub enum PlayerMove {
    Single(Action),
    Stack { value: u8, shapes: Vec<Shape> },
}

pub struct HumanMove {
    pub user_id: Uuid,
    pub mv: PlayerMove,
    pub respond: oneshot::Sender<Result<(), String>>,
}

pub type PlayerTxMap = DashMap<Uuid, mpsc::UnboundedSender<Arc<ServerEvent>>>;

pub struct RoomHandle {
    pub move_tx: mpsc::Sender<HumanMove>,
    pub player_txs: Arc<PlayerTxMap>, // seat holders — get personalised hand
    pub spectator_txs: Arc<PlayerTxMap>, // observers — all hands hidden
    pub human_seat_ids: Arc<HashSet<Uuid>>, // who is a seat holder
}

pub type RoomRegistry = DashMap<Uuid, RoomHandle>;

// ── Wire types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WsAction {
    Suit { shape: Shape, value: u8 },
    Whot { called_shape: Shape },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
// The Set* variants' `enabled` flags are part of the client protocol but not
// yet consumed server-side (media/chat toggles are handled peer-to-peer).
#[allow(dead_code)]
enum ClientEvent {
    PlayCard { action: WsAction },
    PlayStack { value: u8, shapes: Vec<Shape> },
    Draw,
    RtcOffer { to: Uuid, sdp: String },
    RtcAnswer { to: Uuid, sdp: String },
    RtcIce { to: Uuid, candidate: String },
    ChatMessage { text: String },
    SetVideo { enabled: bool },
    SetAudio { enabled: bool },
    SetChat { enabled: bool },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    GameState {
        state: GameStateView,
    },
    GameOver {
        winner_index: Option<usize>,
        winner_name: Option<String>,
    },
    Error {
        message: String,
    },
    RtcSignal {
        from: Uuid,
        kind: String,
        payload: String,
    },
    ChatMessage {
        from: Uuid,
        text: String,
    },
}

// ── Game driver ────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn run_game_driver(
    game_id: Uuid,
    mut state: GameState,
    mut move_rx: mpsc::Receiver<HumanMove>,
    player_txs: Arc<PlayerTxMap>,
    spectator_txs: Arc<PlayerTxMap>,
    redis_client: redis::Client,
    db: sqlx::PgPool,
    rooms: Arc<RoomRegistry>,
) {
    let Ok(mut redis) = redis_client.get_multiplexed_tokio_connection().await else {
        tracing::error!(%game_id, "game driver: cannot connect to Redis");
        return;
    };

    loop {
        let seat_idx = state.current_seat_index;
        let seat_kind = state.seats[seat_idx].kind.clone();

        match seat_kind {
            SeatKind::Human { user_id } => loop {
                let Some(mv) = move_rx.recv().await else {
                    tracing::info!(%game_id, "move channel closed — room cleaned up");
                    return;
                };

                if mv.user_id != user_id {
                    let _ = mv.respond.send(Err("not your turn".into()));
                    continue;
                }

                let result = match &mv.mv {
                    PlayerMove::Single(action) => apply_action(&mut state, seat_idx, *action),
                    PlayerMove::Stack { value, shapes } => {
                        apply_stack(&mut state, seat_idx, *value, shapes)
                    }
                }
                .map_err(|e: GameError| e.to_string());
                let ok = result.is_ok();
                let _ = mv.respond.send(result);
                if ok {
                    break;
                }
            },

            SeatKind::Ai { difficulty } => {
                tokio::time::sleep(Duration::from_millis(800)).await;
                // ISMCTS is CPU-bound (up to ~200ms for TeeNoble); run it off the
                // async worker so it doesn't stall other games on this thread.
                let snapshot = state.clone();
                let mv = tokio::task::spawn_blocking(move || {
                    select_move(&snapshot, seat_idx, difficulty)
                })
                .await
                .unwrap_or(AiMove::Act(Action::Draw));
                if let Err(e) = apply_ai_move(&mut state, seat_idx, mv) {
                    tracing::error!(%game_id, seat = seat_idx, "AI invalid move: {e}");
                    let _ = apply_action(&mut state, seat_idx, Action::Draw);
                }
            }
        }

        if let Err(e) = game_store::save(&mut redis, &state).await {
            tracing::warn!(%game_id, "Redis save failed: {e}");
        }
        touch_game(&db, game_id).await;

        let game_over = state.phase == GamePhase::Finished;
        let winner_index = state.winner_index;
        let winner_name = winner_index
            .and_then(|i| state.seats.get(i))
            .map(|s| s.name.clone());

        broadcast_views(&state, &player_txs, &spectator_txs);

        if game_over {
            let ev = Arc::new(ServerEvent::GameOver {
                winner_index,
                winner_name,
            });
            broadcast_raw(&player_txs, Arc::clone(&ev));
            broadcast_raw(&spectator_txs, ev);
            if let Err(e) = save_result(&db, &state).await {
                tracing::warn!(%game_id, "DB result save failed: {e}");
            }
            let _ = game_store::delete(&mut redis, game_id).await;
            rooms.remove(&game_id);
            break;
        }
    }
}

fn broadcast_views(state: &GameState, player_txs: &PlayerTxMap, spectator_txs: &PlayerTxMap) {
    for entry in player_txs.iter() {
        let view = make_view(state, Some(*entry.key()));
        let _ = entry
            .value()
            .send(Arc::new(ServerEvent::GameState { state: view }));
    }
    if !spectator_txs.is_empty() {
        let sv = Arc::new(ServerEvent::GameState {
            state: make_view(state, None),
        });
        for entry in spectator_txs.iter() {
            let _ = entry.value().send(Arc::clone(&sv));
        }
    }
}

fn broadcast_raw(map: &PlayerTxMap, ev: Arc<ServerEvent>) {
    for entry in map.iter() {
        let _ = entry.value().send(Arc::clone(&ev));
    }
}

async fn touch_game(db: &sqlx::PgPool, game_id: Uuid) {
    if let Err(e) = sqlx::query("UPDATE games SET last_activity_at = NOW() WHERE id = $1")
        .bind(game_id)
        .execute(db)
        .await
    {
        tracing::warn!(%game_id, "touch_game failed: {e}");
    }
}

async fn save_result(db: &sqlx::PgPool, state: &GameState) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE games SET status = 'finished', winner_seat = $2, finished_at = NOW() WHERE id = $1",
    )
    .bind(state.id)
    .bind(state.winner_index.map(|i| i as i32))
    .execute(db)
    .await?;

    if let Some(winner) = state.winner_index {
        sqlx::query(
            "UPDATE game_seats SET is_winner = TRUE WHERE game_id = $1 AND seat_index = $2",
        )
        .bind(state.id)
        .bind(winner as i32)
        .execute(db)
        .await?;
    }

    Ok(())
}

// ── WebSocket upgrade ──────────────────────────────────────────────────────────

pub async fn game_socket(
    ws: WebSocketUpgrade,
    WsAuthUser(claims): WsAuthUser,
    State(app): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, claims.sub, game_id, app)))
}

async fn handle_socket(mut socket: WebSocket, user_id: Uuid, game_id: Uuid, app: AppState) {
    tracing::info!(%user_id, %game_id, "WebSocket connected");

    let access = match ensure_room(game_id, &app).await {
        Ok(a) => a,
        Err(e) => {
            let json = format!("{{\"type\":\"error\",\"message\":\"{e}\"}}");
            let _ = socket.send(Message::Text(json)).await;
            return;
        }
    };

    let is_participant = access.human_seat_ids.contains(&user_id);
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<Arc<ServerEvent>>();

    if is_participant {
        access.player_txs.insert(user_id, ev_tx);
    } else {
        access.spectator_txs.insert(user_id, ev_tx);
    }

    // Send personalised (or spectator) snapshot immediately
    if let Ok(mut redis) = app.redis.get_multiplexed_tokio_connection().await {
        if let Ok(Some(gs)) = game_store::load(&mut redis, game_id).await {
            let view = make_view(&gs, if is_participant { Some(user_id) } else { None });
            if let Ok(json) = serde_json::to_string(&ServerEvent::GameState { state: view }) {
                let _ = socket.send(Message::Text(json)).await;
            }
        }
    }

    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(30),
        Duration::from_secs(30),
    );

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(vec![])).await.is_err() { break; }
            }

            event = ev_rx.recv() => {
                match event {
                    Some(ev) => {
                        if let Ok(json) = serde_json::to_string(ev.as_ref()) {
                            if socket.send(Message::Text(json)).await.is_err() { break; }
                        }
                    }
                    None => break,
                }
            }

            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if is_participant {
                            on_client_message(
                                &text, user_id, &access.move_tx,
                                &access.player_txs, &access.spectator_txs,
                                &app.db, &mut socket,
                            )
                            .await;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    if is_participant {
        access.player_txs.remove(&user_id);
    } else {
        access.spectator_txs.remove(&user_id);
    }
    tracing::info!(%user_id, %game_id, "WebSocket disconnected");
}

async fn on_client_message(
    text: &str,
    user_id: Uuid,
    move_tx: &mpsc::Sender<HumanMove>,
    player_txs: &Arc<PlayerTxMap>,
    spectator_txs: &Arc<PlayerTxMap>,
    db: &sqlx::PgPool,
    socket: &mut WebSocket,
) {
    let Ok(event) = serde_json::from_str::<ClientEvent>(text) else {
        return;
    };

    match event {
        ClientEvent::PlayCard { action } => {
            let action = match action {
                WsAction::Suit { shape, value } => Action::PlaySuit { shape, value },
                WsAction::Whot { called_shape } => Action::PlayWhot { called_shape },
            };
            send_move(user_id, PlayerMove::Single(action), move_tx, socket).await;
        }
        ClientEvent::PlayStack { value, shapes } => {
            send_move(user_id, PlayerMove::Stack { value, shapes }, move_tx, socket).await;
        }
        ClientEvent::Draw => {
            send_move(user_id, PlayerMove::Single(Action::Draw), move_tx, socket).await
        }

        ClientEvent::RtcOffer { to, sdp } => {
            relay_rtc(user_id, to, "offer", sdp, player_txs, db).await
        }
        ClientEvent::RtcAnswer { to, sdp } => {
            relay_rtc(user_id, to, "answer", sdp, player_txs, db).await
        }
        ClientEvent::RtcIce { to, candidate } => {
            relay_rtc(user_id, to, "ice", candidate, player_txs, db).await
        }

        ClientEvent::ChatMessage { text } => {
            let ev = Arc::new(ServerEvent::ChatMessage {
                from: user_id,
                text,
            });
            broadcast_raw(player_txs, Arc::clone(&ev));
            broadcast_raw(spectator_txs, ev);
        }

        ClientEvent::SetVideo { .. }
        | ClientEvent::SetAudio { .. }
        | ClientEvent::SetChat { .. } => {}
    }
}

async fn relay_rtc(
    from: Uuid,
    to: Uuid,
    kind: &str,
    payload: String,
    player_txs: &Arc<PlayerTxMap>,
    db: &sqlx::PgPool,
) {
    if !are_friends(db, from, to).await {
        return;
    }
    if let Some(tx) = player_txs.get(&to) {
        let _ = tx.send(Arc::new(ServerEvent::RtcSignal {
            from,
            kind: kind.into(),
            payload,
        }));
    }
}

async fn are_friends(db: &sqlx::PgPool, a: Uuid, b: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM friends
            WHERE status = 'accepted'
              AND ((requester_id = $1 AND addressee_id = $2)
                OR (requester_id = $2 AND addressee_id = $1))
        )",
    )
    .bind(a)
    .bind(b)
    .fetch_one(db)
    .await
    .unwrap_or(false)
}

async fn send_move(
    user_id: Uuid,
    mv: PlayerMove,
    move_tx: &mpsc::Sender<HumanMove>,
    socket: &mut WebSocket,
) {
    let (tx, rx) = oneshot::channel();
    if move_tx
        .send(HumanMove {
            user_id,
            mv,
            respond: tx,
        })
        .await
        .is_err()
    {
        return;
    }
    if let Ok(Err(msg)) = rx.await {
        let json = format!("{{\"type\":\"error\",\"message\":\"{msg}\"}}");
        let _ = socket.send(Message::Text(json)).await;
    }
}

// ── Room access returned by ensure_room ────────────────────────────────────────

struct RoomAccess {
    move_tx: mpsc::Sender<HumanMove>,
    player_txs: Arc<PlayerTxMap>,
    spectator_txs: Arc<PlayerTxMap>,
    human_seat_ids: Arc<HashSet<Uuid>>,
}

async fn ensure_room(game_id: Uuid, app: &AppState) -> anyhow::Result<RoomAccess> {
    if let Some(h) = app.rooms.get(&game_id) {
        return Ok(RoomAccess {
            move_tx: h.move_tx.clone(),
            player_txs: Arc::clone(&h.player_txs),
            spectator_txs: Arc::clone(&h.spectator_txs),
            human_seat_ids: Arc::clone(&h.human_seat_ids),
        });
    }

    let mut redis = app.redis.get_multiplexed_tokio_connection().await?;
    let game_state = game_store::load(&mut redis, game_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("game not found"))?;

    let human_seat_ids: HashSet<Uuid> = game_state
        .seats
        .iter()
        .filter_map(|s| match &s.kind {
            SeatKind::Human { user_id } => Some(*user_id),
            SeatKind::Ai { .. } => None,
        })
        .collect();
    let human_seat_ids = Arc::new(human_seat_ids);

    let (move_tx, move_rx) = mpsc::channel::<HumanMove>(64);
    let player_txs: Arc<PlayerTxMap> = Arc::new(DashMap::new());
    let spectator_txs: Arc<PlayerTxMap> = Arc::new(DashMap::new());

    app.rooms.insert(
        game_id,
        RoomHandle {
            move_tx: move_tx.clone(),
            player_txs: Arc::clone(&player_txs),
            spectator_txs: Arc::clone(&spectator_txs),
            human_seat_ids: Arc::clone(&human_seat_ids),
        },
    );

    tokio::spawn(run_game_driver(
        game_id,
        game_state,
        move_rx,
        Arc::clone(&player_txs),
        Arc::clone(&spectator_txs),
        app.redis.clone(),
        app.db.clone(),
        Arc::clone(&app.rooms),
    ));

    Ok(RoomAccess {
        move_tx,
        player_txs,
        spectator_txs,
        human_seat_ids,
    })
}
