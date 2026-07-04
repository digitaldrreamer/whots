use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    game::{
        engine::create_game,
        types::{Difficulty, GameMode, Seat, SeatKind},
    },
    models::user::PublicUser,
    state::AppState,
    store::{game_store, notification_store},
};

// ── DELETE /games/:id ──────────────────────────────────────────────────────────

pub async fn cancel(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let is_participant: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_seats WHERE game_id = $1 AND user_id = $2)",
    )
    .bind(game_id)
    .bind(claims.sub)
    .fetch_one(&app.db)
    .await?;

    if !is_participant {
        return Err(AppError::Forbidden);
    }

    let updated = sqlx::query_scalar::<_, Uuid>(
        "UPDATE games SET status = 'abandoned'
         WHERE id = $1 AND status IN ('playing', 'waiting')
         RETURNING id",
    )
    .bind(game_id)
    .fetch_optional(&app.db)
    .await?;

    if updated.is_none() {
        return Err(AppError::NotFound(
            "game not found or already finished".into(),
        ));
    }

    app.rooms.remove(&game_id);

    if let Ok(mut conn) = app.redis.get_multiplexed_tokio_connection().await {
        let _ = game_store::delete(&mut conn, game_id).await;
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /games ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatSpec {
    // Human seats carry only a user id — the display name is resolved
    // server-side from the users table, never taken from the client.
    Human {
        user_id: Uuid,
    },
    Ai {
        difficulty: Difficulty,
        name: String,
    },
}

#[derive(Deserialize)]
pub struct CreateGameRequest {
    pub mode: GameMode,
    pub seats: Vec<SeatSpec>,
}

#[derive(Serialize)]
pub struct CreateGameResponse {
    pub game_id: Uuid,
}

pub async fn create(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Json(body): Json<CreateGameRequest>,
) -> Result<(StatusCode, Json<CreateGameResponse>), AppError> {
    if body.seats.len() < 2 {
        return Err(AppError::BadRequest("need at least 2 seats".into()));
    }
    if body.seats.len() > 6 {
        return Err(AppError::BadRequest("maximum 6 seats".into()));
    }

    let caller_present = body
        .seats
        .iter()
        .any(|s| matches!(s, SeatSpec::Human { user_id } if *user_id == claims.sub));
    if !caller_present {
        return Err(AppError::BadRequest(
            "you must be a participant in the game you create".into(),
        ));
    }

    // Resolve every human seat's display name from the database rather than
    // trusting the caller-supplied `name`. Otherwise the game creator could set
    // an arbitrary name on another user's seat (impersonation / content
    // injection), which then gets stored, broadcast over WebSocket and pushed
    // as a notification to the victim. This also validates that each referenced
    // user actually exists.
    let human_ids: Vec<Uuid> = body
        .seats
        .iter()
        .filter_map(|s| match s {
            SeatSpec::Human { user_id } => Some(*user_id),
            SeatSpec::Ai { .. } => None,
        })
        .collect();

    let display_names: HashMap<Uuid, String> = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, display_name FROM users WHERE id = ANY($1)",
    )
    .bind(&human_ids)
    .fetch_all(&app.db)
    .await?
    .into_iter()
    .collect();

    for id in &human_ids {
        if !display_names.contains_key(id) {
            return Err(AppError::BadRequest("unknown user in seats".into()));
        }
    }

    let seats: Vec<Seat> = body
        .seats
        .iter()
        .map(|spec| match spec {
            SeatSpec::Human { user_id } => Seat {
                // Server-resolved name — client input is ignored for human seats.
                name: display_names.get(user_id).cloned().unwrap_or_default(),
                kind: SeatKind::Human { user_id: *user_id },
                hand: vec![],
                owed_draws: 0,
            },
            SeatSpec::Ai { difficulty, name } => Seat {
                name: name.clone(),
                kind: SeatKind::Ai {
                    difficulty: *difficulty,
                },
                hand: vec![],
                owed_draws: 0,
            },
        })
        .collect();

    let game_id = persist_new_game(&app, body.mode, seats, claims.sub).await?;

    // Notify every other human seat of the invite
    for spec in &body.seats {
        if let SeatSpec::Human { user_id } = spec {
            if *user_id != claims.sub {
                notification_store::push(
                    &app.db,
                    &app.notify_txs,
                    *user_id,
                    "game_invite",
                    serde_json::json!({ "game_id": game_id, "from_username": claims.username }),
                )
                .await;
            }
        }
    }

    Ok((StatusCode::CREATED, Json(CreateGameResponse { game_id })))
}

/// Deal a game from already-resolved seats, persist it (Postgres `games` +
/// `game_seats`, Redis snapshot), and return its id. Shared by `POST /games` and
/// the lobby's `POST /rooms/:id/start`. Does NOT send invites/notifications —
/// each caller does its own fan-out.
pub async fn persist_new_game(
    app: &AppState,
    mode: GameMode,
    seats: Vec<Seat>,
    created_by: Uuid,
) -> Result<Uuid, AppError> {
    let game_state = create_game(seats, mode);
    let game_id = game_state.id;

    sqlx::query("INSERT INTO games (id, mode, status, created_by) VALUES ($1, $2, 'playing', $3)")
        .bind(game_id)
        .bind(mode.to_db_str())
        .bind(created_by)
        .execute(&app.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Seat order is preserved by create_game, so game_state.seats[idx] is seat idx.
    for (idx, seat) in game_state.seats.iter().enumerate() {
        let (uid, is_ai, difficulty_str) = match &seat.kind {
            SeatKind::Human { user_id } => (Some(*user_id), false, None),
            SeatKind::Ai { difficulty } => (None, true, Some(difficulty.to_db_str())),
        };
        sqlx::query(
            "INSERT INTO game_seats (game_id, seat_index, user_id, is_ai, ai_difficulty)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(game_id)
        .bind(idx as i32)
        .bind(uid)
        .bind(is_ai)
        .bind(difficulty_str)
        .execute(&app.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    }

    let mut redis = app
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    game_store::save(&mut redis, &game_state)
        .await
        .map_err(AppError::Internal)?;

    Ok(game_id)
}

// ── GET /games/:id ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct GameSeatRow {
    id: Uuid,
    mode: String,
    status: String,
    created_by: Option<Uuid>,
    created_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    seat_index: Option<i32>,
    seat_user_id: Option<Uuid>,
    is_ai: Option<bool>,
    ai_difficulty: Option<String>,
    is_winner: Option<bool>,
    username: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    is_guest: Option<bool>,
}

#[derive(Serialize)]
pub struct SeatResponse {
    pub seat_index: i32,
    pub is_ai: bool,
    pub ai_difficulty: Option<String>,
    pub is_winner: bool,
    pub user: Option<PublicUser>,
}

#[derive(Serialize)]
pub struct GameResponse {
    pub id: Uuid,
    pub mode: String,
    pub status: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub seats: Vec<SeatResponse>,
}

pub async fn get_by_id(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<GameResponse>, AppError> {
    // Only participants (seat holders) or the game's creator may view its
    // details. Without this, any authenticated user could enumerate game UUIDs
    // and harvest every player's user id, username and avatar.
    let (exists, is_participant): (bool, bool) = sqlx::query_as(
        "SELECT
             EXISTS(SELECT 1 FROM games WHERE id = $1) AS game_exists,
             EXISTS(
                 SELECT 1 FROM game_seats WHERE game_id = $1 AND user_id = $2
                 UNION ALL
                 SELECT 1 FROM games      WHERE id = $1      AND created_by = $2
             ) AS is_participant",
    )
    .bind(game_id)
    .bind(claims.sub)
    .fetch_one(&app.db)
    .await?;

    if !exists {
        return Err(AppError::NotFound("game not found".into()));
    }
    if !is_participant {
        return Err(AppError::Forbidden);
    }

    let rows = sqlx::query_as::<_, GameSeatRow>(
        "SELECT
             g.id, g.mode, g.status, g.created_by, g.created_at, g.finished_at,
             gs.seat_index,
             gs.user_id   AS seat_user_id,
             gs.is_ai,
             gs.ai_difficulty,
             gs.is_winner,
             u.username, u.display_name, u.avatar_url, u.is_guest
         FROM games g
         LEFT JOIN game_seats gs ON gs.game_id = g.id
         LEFT JOIN users u       ON u.id = gs.user_id
         WHERE g.id = $1
         ORDER BY gs.seat_index",
    )
    .bind(game_id)
    .fetch_all(&app.db)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound("game not found".into()));
    }

    let first = &rows[0];
    let seats = rows
        .iter()
        .filter_map(|r| {
            let idx = r.seat_index?;
            Some(SeatResponse {
                seat_index: idx,
                is_ai: r.is_ai.unwrap_or(false),
                ai_difficulty: r.ai_difficulty.clone(),
                is_winner: r.is_winner.unwrap_or(false),
                user: r.seat_user_id.map(|uid| PublicUser {
                    id: uid,
                    username: r.username.clone().unwrap_or_default(),
                    display_name: r.display_name.clone().unwrap_or_default(),
                    avatar_url: r.avatar_url.clone(),
                    is_guest: r.is_guest.unwrap_or(false),
                }),
            })
        })
        .collect();

    Ok(Json(GameResponse {
        id: first.id,
        mode: first.mode.clone(),
        status: first.status.clone(),
        created_by: first.created_by,
        created_at: first.created_at,
        finished_at: first.finished_at,
        seats,
    }))
}

// ── POST /games/:id/accept ────────────────────────────────────────────────────

pub async fn accept(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let rows_updated = sqlx::query(
        "UPDATE game_seats SET accepted_at = NOW()
         WHERE game_id = $1 AND user_id = $2 AND is_ai = FALSE AND accepted_at IS NULL",
    )
    .bind(game_id)
    .bind(claims.sub)
    .execute(&app.db)
    .await
    .map_err(|e| AppError::Internal(e.into()))?
    .rows_affected();

    if rows_updated == 0 {
        return Err(AppError::NotFound(
            "no pending invite for you in this game".into(),
        ));
    }

    let creator_id: Option<Uuid> = sqlx::query_scalar("SELECT created_by FROM games WHERE id = $1")
        .bind(game_id)
        .fetch_optional(&app.db)
        .await?;

    if let Some(cid) = creator_id {
        if cid != claims.sub {
            notification_store::push(
                &app.db,
                &app.notify_txs,
                cid,
                "game_accepted",
                serde_json::json!({
                    "game_id": game_id,
                    "from_username": claims.username
                }),
            )
            .await;
        }
    }

    Ok(Json(serde_json::json!({ "game_id": game_id })))
}

// ── POST /games/:id/decline ───────────────────────────────────────────────────

pub async fn decline(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let is_seat: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_seats WHERE game_id = $1 AND user_id = $2 AND is_ai = FALSE)",
    )
    .bind(game_id)
    .bind(claims.sub)
    .fetch_one(&app.db)
    .await?;

    if !is_seat {
        return Err(AppError::Forbidden);
    }

    let updated = sqlx::query_scalar::<_, Uuid>(
        "UPDATE games SET status = 'abandoned'
         WHERE id = $1 AND status IN ('playing', 'waiting')
         RETURNING id",
    )
    .bind(game_id)
    .fetch_optional(&app.db)
    .await?;

    if updated.is_none() {
        return Err(AppError::NotFound(
            "game not found or already finished".into(),
        ));
    }

    app.rooms.remove(&game_id);
    if let Ok(mut conn) = app.redis.get_multiplexed_tokio_connection().await {
        let _ = game_store::delete(&mut conn, game_id).await;
    }

    let creator_id: Option<Uuid> = sqlx::query_scalar("SELECT created_by FROM games WHERE id = $1")
        .bind(game_id)
        .fetch_optional(&app.db)
        .await?;

    if let Some(cid) = creator_id {
        if cid != claims.sub {
            notification_store::push(
                &app.db,
                &app.notify_txs,
                cid,
                "game_declined",
                serde_json::json!({
                    "game_id": game_id,
                    "from_username": claims.username
                }),
            )
            .await;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── GameMode helper ────────────────────────────────────────────────────────────

impl GameMode {
    fn to_db_str(self) -> &'static str {
        match self {
            GameMode::Stack => "stack",
            GameMode::NoStack => "no_stack",
        }
    }
}
