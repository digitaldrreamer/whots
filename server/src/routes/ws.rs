use std::{sync::Arc, time::Duration};

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    response::IntoResponse,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    game::{
        ai::select_move,
        engine::{apply_action, make_view, GameError},
        types::{Action, GamePhase, GameState, GameStateView, SeatKind, Shape},
    },
    state::AppState,
    store::game_store,
};

// ── Room registry ──────────────────────────────────────────────────────────────

pub struct HumanMove {
    pub user_id: Uuid,
    pub action:  Action,
    pub respond: oneshot::Sender<Result<(), String>>,
}

pub type PlayerTxMap = DashMap<Uuid, mpsc::UnboundedSender<Arc<ServerEvent>>>;

pub struct RoomHandle {
    pub move_tx:    mpsc::Sender<HumanMove>,
    pub player_txs: Arc<PlayerTxMap>,
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
enum ClientEvent {
    PlayCard    { action: WsAction },
    Draw,
    RtcOffer    { to: Uuid, sdp: String },
    RtcAnswer   { to: Uuid, sdp: String },
    RtcIce      { to: Uuid, candidate: String },
    ChatMessage { text: String },
    SetVideo    { enabled: bool },
    SetAudio    { enabled: bool },
    SetChat     { enabled: bool },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    GameState   { state: GameStateView },
    GameOver    { winner_index: Option<usize>, winner_name: Option<String> },
    Error       { message: String },
    RtcSignal   { from: Uuid, kind: String, payload: String },
    ChatMessage { from: Uuid, text: String },
}

// ── Game driver ────────────────────────────────────────────────────────────────

pub async fn run_game_driver(
    game_id: Uuid,
    mut state: GameState,
    mut move_rx: mpsc::Receiver<HumanMove>,
    player_txs: Arc<PlayerTxMap>,
    redis_client: redis::Client,
    db: sqlx::PgPool,
    rooms: Arc<RoomRegistry>,
) {
    let Ok(mut redis) = redis_client.get_multiplexed_tokio_connection().await else {
        tracing::error!(%game_id, "game driver: cannot connect to Redis");
        return;
    };

    loop {
        let seat_idx  = state.current_seat_index;
        let seat_kind = state.seats[seat_idx].kind.clone();

        match seat_kind {
            SeatKind::Human { user_id } => {
                loop {
                    let Some(mv) = move_rx.recv().await else {
                        tracing::info!(%game_id, "all players disconnected");
                        rooms.remove(&game_id);
                        return;
                    };

                    if mv.user_id != user_id {
                        let _ = mv.respond.send(Err("not your turn".into()));
                        continue;
                    }

                    let result = apply_action(&mut state, seat_idx, mv.action)
                        .map_err(|e: GameError| e.to_string());
                    let ok = result.is_ok();
                    let _ = mv.respond.send(result);
                    if ok { break; }
                }
            }

            SeatKind::Ai { difficulty } => {
                tokio::time::sleep(Duration::from_millis(800)).await;
                let action = select_move(&state, seat_idx, difficulty);
                if let Err(e) = apply_action(&mut state, seat_idx, action) {
                    tracing::error!(%game_id, seat = seat_idx, "AI invalid move: {e}");
                    let _ = apply_action(&mut state, seat_idx, Action::Draw);
                }
            }
        }

        // Persist state and update activity timestamp
        if let Err(e) = game_store::save(&mut redis, &state).await {
            tracing::warn!(%game_id, "Redis save failed: {e}");
        }
        touch_game(&db, game_id).await;

        let game_over    = state.phase == GamePhase::Finished;
        let winner_index = state.winner_index;
        let winner_name  = winner_index
            .and_then(|i| state.seats.get(i))
            .map(|s| s.name.clone());

        // Send each connected player their own personalised view
        broadcast_views(&state, &player_txs);

        if game_over {
            broadcast_event(&player_txs, Arc::new(ServerEvent::GameOver { winner_index, winner_name }));
            if let Err(e) = save_result(&db, &state).await {
                tracing::warn!(%game_id, "DB result save failed: {e}");
            }
            let _ = game_store::delete(&mut redis, game_id).await;
            rooms.remove(&game_id);
            break;
        }
    }
}

fn broadcast_views(state: &GameState, player_txs: &PlayerTxMap) {
    for entry in player_txs.iter() {
        let uid  = *entry.key();
        let view = make_view(state, Some(uid));
        let _    = entry.value().send(Arc::new(ServerEvent::GameState { state: view }));
    }
}

fn broadcast_event(player_txs: &PlayerTxMap, ev: Arc<ServerEvent>) {
    for entry in player_txs.iter() {
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
    Ok(())
}

// ── WebSocket upgrade ──────────────────────────────────────────────────────────

pub async fn game_socket(
    ws: WebSocketUpgrade,
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, claims.sub, game_id, app)))
}

async fn handle_socket(mut socket: WebSocket, user_id: Uuid, game_id: Uuid, app: AppState) {
    tracing::info!(%user_id, %game_id, "WebSocket connected");

    let (move_tx, player_txs) = match ensure_room(game_id, &app).await {
        Ok(pair) => pair,
        Err(e) => {
            let json = format!("{{\"type\":\"error\",\"message\":\"{e}\"}}");
            let _ = socket.send(Message::Text(json.into())).await;
            return;
        }
    };

    // Register a personal channel for this player
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel::<Arc<ServerEvent>>();
    player_txs.insert(user_id, ev_tx);

    // Send the current state snapshot immediately (personalised)
    if let Ok(mut redis) = app.redis.get_multiplexed_tokio_connection().await {
        if let Ok(Some(gs)) = game_store::load(&mut redis, game_id).await {
            let view = make_view(&gs, Some(user_id));
            if let Ok(json) = serde_json::to_string(&ServerEvent::GameState { state: view }) {
                let _ = socket.send(Message::Text(json.into())).await;
            }
        }
    }

    loop {
        tokio::select! {
            event = ev_rx.recv() => {
                match event {
                    Some(ev) => {
                        if let Ok(json) = serde_json::to_string(ev.as_ref()) {
                            if socket.send(Message::Text(json.into())).await.is_err() { break; }
                        }
                    }
                    None => break,
                }
            }

            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        on_client_message(&text, user_id, &move_tx, &player_txs, &app.db, &mut socket).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    player_txs.remove(&user_id);
    tracing::info!(%user_id, %game_id, "WebSocket disconnected");
}

async fn on_client_message(
    text: &str,
    user_id: Uuid,
    move_tx: &mpsc::Sender<HumanMove>,
    player_txs: &Arc<PlayerTxMap>,
    db: &sqlx::PgPool,
    socket: &mut WebSocket,
) {
    let Ok(event) = serde_json::from_str::<ClientEvent>(text) else { return };

    match event {
        ClientEvent::PlayCard { action } => {
            let action = match action {
                WsAction::Suit { shape, value } => Action::PlaySuit { shape, value },
                WsAction::Whot { called_shape } => Action::PlayWhot { called_shape },
            };
            send_move(user_id, action, move_tx, socket).await;
        }
        ClientEvent::Draw => send_move(user_id, Action::Draw, move_tx, socket).await,

        ClientEvent::RtcOffer   { to, sdp }       => relay_rtc(user_id, to, "offer",  sdp,       player_txs, db).await,
        ClientEvent::RtcAnswer  { to, sdp }       => relay_rtc(user_id, to, "answer", sdp,       player_txs, db).await,
        ClientEvent::RtcIce     { to, candidate } => relay_rtc(user_id, to, "ice",    candidate, player_txs, db).await,

        ClientEvent::ChatMessage { text } => {
            broadcast_event(player_txs, Arc::new(ServerEvent::ChatMessage { from: user_id, text }));
        }

        ClientEvent::SetVideo { .. } | ClientEvent::SetAudio { .. } | ClientEvent::SetChat { .. } => {}
    }
}

// Gate WebRTC relay on mutual friendship; send directly to the target player only.
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
        let ev = Arc::new(ServerEvent::RtcSignal { from, kind: kind.to_string(), payload });
        let _ = tx.send(ev);
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
    action: Action,
    move_tx: &mpsc::Sender<HumanMove>,
    socket: &mut WebSocket,
) {
    let (tx, rx) = oneshot::channel();
    if move_tx.send(HumanMove { user_id, action, respond: tx }).await.is_err() {
        return;
    }
    if let Ok(Err(msg)) = rx.await {
        let json = format!("{{\"type\":\"error\",\"message\":\"{msg}\"}}");
        let _ = socket.send(Message::Text(json.into())).await;
    }
}

async fn ensure_room(
    game_id: Uuid,
    app: &AppState,
) -> anyhow::Result<(mpsc::Sender<HumanMove>, Arc<PlayerTxMap>)> {
    if let Some(h) = app.rooms.get(&game_id) {
        return Ok((h.move_tx.clone(), Arc::clone(&h.player_txs)));
    }

    let mut redis = app.redis.get_multiplexed_tokio_connection().await?;
    let game_state = game_store::load(&mut redis, game_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("game not found"))?;

    let (move_tx, move_rx) = mpsc::channel::<HumanMove>(64);
    let player_txs: Arc<PlayerTxMap> = Arc::new(DashMap::new());

    app.rooms.insert(game_id, RoomHandle {
        move_tx:    move_tx.clone(),
        player_txs: Arc::clone(&player_txs),
    });

    tokio::spawn(run_game_driver(
        game_id,
        game_state,
        move_rx,
        Arc::clone(&player_txs),
        app.redis.clone(),
        app.db.clone(),
        Arc::clone(&app.rooms),
    ));

    Ok((move_tx, player_txs))
}
