use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    game::types::{Difficulty, GameMode, Seat, SeatKind},
    routes::games::persist_new_game,
    state::AppState,
    store::{
        lobby_store::{self, Lobby, LobbyMember},
        notification_store,
    },
};

const MAX_SLOTS: usize = 6;
const AI_NAMES: [&str; 5] = ["Ada", "Emeka", "Ngozi", "Bisi", "Tunde"];

// ── helpers ─────────────────────────────────────────────────────────────────

async fn redis_conn(app: &AppState) -> Result<redis::aio::MultiplexedConnection, AppError> {
    app.redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| AppError::Internal(e.into()))
}

async fn load_lobby(app: &AppState, id: Uuid) -> Result<Lobby, AppError> {
    let mut conn = redis_conn(app).await?;
    lobby_store::load(&mut conn, id)
        .await
        .map_err(AppError::Internal)?
        .ok_or_else(|| AppError::NotFound("room not found".into()))
}

async fn save_lobby(app: &AppState, lobby: &Lobby) -> Result<(), AppError> {
    let mut conn = redis_conn(app).await?;
    lobby_store::save(&mut conn, lobby)
        .await
        .map_err(AppError::Internal)
}

/// Live-nudge every member (their client re-fetches GET /rooms/:id).
async fn broadcast(app: &AppState, lobby: &Lobby, kind: &str) {
    for m in &lobby.members {
        notification_store::push(
            &app.db,
            &app.notify_txs,
            m.user_id,
            kind,
            serde_json::json!({ "room_id": lobby.id }),
        )
        .await;
    }
}

fn is_member(lobby: &Lobby, user_id: Uuid) -> bool {
    lobby.members.iter().any(|m| m.user_id == user_id)
}

fn slots_used(lobby: &Lobby) -> usize {
    lobby.members.len() + lobby.ais.len()
}

// ── POST /rooms ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub mode: GameMode,
}

#[derive(Serialize)]
pub struct CreateRoomResponse {
    pub room_id: Uuid,
}

pub async fn create(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Json(body): Json<CreateRoomRequest>,
) -> Result<(StatusCode, Json<CreateRoomResponse>), AppError> {
    let id = Uuid::new_v4();
    let lobby = Lobby {
        id,
        host_id: claims.sub,
        mode: body.mode,
        members: vec![LobbyMember {
            user_id: claims.sub,
            username: claims.username.clone(),
        }],
        ais: vec![],
        invited: vec![],
    };
    save_lobby(&app, &lobby).await?;
    Ok((StatusCode::CREATED, Json(CreateRoomResponse { room_id: id })))
}

// ── GET /rooms/:id ──────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RoomView {
    pub id: Uuid,
    pub host_id: Uuid,
    pub am_i_host: bool,
    pub mode: GameMode,
    pub members: Vec<LobbyMember>,
    pub ais: Vec<Difficulty>,
    pub max_slots: usize,
}

pub async fn get_room(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RoomView>, AppError> {
    let lobby = load_lobby(&app, id).await?;
    if !is_member(&lobby, claims.sub) && !lobby.invited.contains(&claims.sub) {
        return Err(AppError::Forbidden);
    }
    Ok(Json(RoomView {
        id: lobby.id,
        host_id: lobby.host_id,
        am_i_host: lobby.host_id == claims.sub,
        mode: lobby.mode,
        members: lobby.members.clone(),
        ais: lobby.ais.clone(),
        max_slots: MAX_SLOTS,
    }))
}

// ── POST /rooms/:id/invite ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct InviteRequest {
    pub user_id: Uuid,
}

pub async fn invite(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<InviteRequest>,
) -> Result<StatusCode, AppError> {
    let mut lobby = load_lobby(&app, id).await?;
    if lobby.host_id != claims.sub {
        return Err(AppError::Forbidden);
    }
    // Friends-only: you can only pull people you're actually friends with.
    let are_friends: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM friends
         WHERE status = 'accepted'
           AND ((requester_id = $1 AND addressee_id = $2)
             OR (requester_id = $2 AND addressee_id = $1)))",
    )
    .bind(claims.sub)
    .bind(body.user_id)
    .fetch_one(&app.db)
    .await?;
    if !are_friends {
        return Err(AppError::BadRequest("you can only invite friends".into()));
    }

    if !lobby.invited.contains(&body.user_id) && !is_member(&lobby, body.user_id) {
        lobby.invited.push(body.user_id);
        save_lobby(&app, &lobby).await?;
    }
    notification_store::push(
        &app.db,
        &app.notify_txs,
        body.user_id,
        "lobby_invite",
        serde_json::json!({ "room_id": id, "from_username": claims.username }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

// ── POST /rooms/:id/join ────────────────────────────────────────────────────

pub async fn join(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let mut lobby = load_lobby(&app, id).await?;
    if is_member(&lobby, claims.sub) {
        return Ok(StatusCode::NO_CONTENT); // idempotent
    }
    if lobby.host_id != claims.sub && !lobby.invited.contains(&claims.sub) {
        return Err(AppError::Forbidden);
    }
    if slots_used(&lobby) >= MAX_SLOTS {
        return Err(AppError::BadRequest("room is full".into()));
    }
    lobby.members.push(LobbyMember {
        user_id: claims.sub,
        username: claims.username.clone(),
    });
    lobby.invited.retain(|u| *u != claims.sub);
    save_lobby(&app, &lobby).await?;
    broadcast(&app, &lobby, "lobby_update").await;
    Ok(StatusCode::NO_CONTENT)
}

// ── POST /rooms/:id/leave ───────────────────────────────────────────────────

pub async fn leave(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let mut lobby = load_lobby(&app, id).await?;
    if lobby.host_id == claims.sub {
        // Host leaving closes the room for everyone.
        broadcast(&app, &lobby, "lobby_closed").await;
        let mut conn = redis_conn(&app).await?;
        let _ = lobby_store::delete(&mut conn, id).await;
        return Ok(StatusCode::NO_CONTENT);
    }
    lobby.members.retain(|m| m.user_id != claims.sub);
    save_lobby(&app, &lobby).await?;
    broadcast(&app, &lobby, "lobby_update").await;
    Ok(StatusCode::NO_CONTENT)
}

// ── POST /rooms/:id/ai  &  DELETE /rooms/:id/ai/:index ──────────────────────

#[derive(Deserialize)]
pub struct AddAiRequest {
    pub difficulty: Difficulty,
}

pub async fn add_ai(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<AddAiRequest>,
) -> Result<StatusCode, AppError> {
    let mut lobby = load_lobby(&app, id).await?;
    if lobby.host_id != claims.sub {
        return Err(AppError::Forbidden);
    }
    if slots_used(&lobby) >= MAX_SLOTS {
        return Err(AppError::BadRequest("room is full".into()));
    }
    lobby.ais.push(body.difficulty);
    save_lobby(&app, &lobby).await?;
    broadcast(&app, &lobby, "lobby_update").await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_ai(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path((id, index)): Path<(Uuid, usize)>,
) -> Result<StatusCode, AppError> {
    let mut lobby = load_lobby(&app, id).await?;
    if lobby.host_id != claims.sub {
        return Err(AppError::Forbidden);
    }
    if index < lobby.ais.len() {
        lobby.ais.remove(index);
        save_lobby(&app, &lobby).await?;
        broadcast(&app, &lobby, "lobby_update").await;
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── POST /rooms/:id/start ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StartResponse {
    pub game_id: Uuid,
}

pub async fn start(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<StartResponse>, AppError> {
    let lobby = load_lobby(&app, id).await?;
    if lobby.host_id != claims.sub {
        return Err(AppError::Forbidden);
    }
    let total = slots_used(&lobby);
    if total < 2 {
        return Err(AppError::BadRequest("need at least 2 players to start".into()));
    }

    // Resolve display names for the human members (matches POST /games).
    let ids: Vec<Uuid> = lobby.members.iter().map(|m| m.user_id).collect();
    let display_names: HashMap<Uuid, String> =
        sqlx::query_as::<_, (Uuid, String)>("SELECT id, display_name FROM users WHERE id = ANY($1)")
            .bind(&ids)
            .fetch_all(&app.db)
            .await?
            .into_iter()
            .collect();

    // Seat order = joined humans (host first), then AI seats.
    let mut seats: Vec<Seat> = Vec::with_capacity(total);
    for m in &lobby.members {
        seats.push(Seat {
            name: display_names
                .get(&m.user_id)
                .cloned()
                .unwrap_or_else(|| m.username.clone()),
            kind: SeatKind::Human { user_id: m.user_id },
            hand: vec![],
            owed_draws: 0,
        });
    }
    for (i, difficulty) in lobby.ais.iter().enumerate() {
        seats.push(Seat {
            name: AI_NAMES.get(i % AI_NAMES.len()).unwrap_or(&"CPU").to_string(),
            kind: SeatKind::Ai {
                difficulty: *difficulty,
            },
            hand: vec![],
            owed_draws: 0,
        });
    }

    let game_id = persist_new_game(&app, lobby.mode, seats, claims.sub).await?;

    // Send everyone but the host into the game (host uses the sync response).
    for m in &lobby.members {
        if m.user_id != claims.sub {
            notification_store::push(
                &app.db,
                &app.notify_txs,
                m.user_id,
                "game_start",
                serde_json::json!({ "game_id": game_id }),
            )
            .await;
        }
    }

    let mut conn = redis_conn(&app).await?;
    let _ = lobby_store::delete(&mut conn, id).await;

    Ok(Json(StartResponse { game_id }))
}
