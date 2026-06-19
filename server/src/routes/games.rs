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

// ── POST /games ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeatSpec {
    Human { user_id: Uuid, name: String },
    Ai    { difficulty: Difficulty, name: String },
}

#[derive(Deserialize)]
pub struct CreateGameRequest {
    pub mode:  GameMode,
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

    let caller_present = body.seats.iter().any(|s| {
        matches!(s, SeatSpec::Human { user_id, .. } if *user_id == claims.sub)
    });
    if !caller_present {
        return Err(AppError::BadRequest("you must be a participant in the game you create".into()));
    }

    let seats: Vec<Seat> = body
        .seats
        .iter()
        .map(|spec| match spec {
            SeatSpec::Human { user_id, name } => Seat {
                name: name.clone(),
                kind: SeatKind::Human { user_id: *user_id },
                hand: vec![],
            },
            SeatSpec::Ai { difficulty, name } => Seat {
                name: name.clone(),
                kind: SeatKind::Ai { difficulty: *difficulty },
                hand: vec![],
            },
        })
        .collect();

    let game_state = create_game(seats, body.mode);
    let game_id    = game_state.id;

    sqlx::query(
        "INSERT INTO games (id, mode, status, created_by) VALUES ($1, $2, 'playing', $3)",
    )
    .bind(game_id)
    .bind(body.mode.to_db_str())
    .bind(claims.sub)
    .execute(&app.db)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    for (idx, spec) in body.seats.iter().enumerate() {
        let (uid, is_ai, difficulty_str) = match spec {
            SeatSpec::Human { user_id, .. } => (Some(*user_id), false, None),
            SeatSpec::Ai { difficulty, .. }  => (None, true, Some(difficulty.to_db_str())),
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
        .map_err(|e| AppError::Internal(e))?;

    // Notify every other human seat of the invite
    for spec in &body.seats {
        if let SeatSpec::Human { user_id, .. } = spec {
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

// ── GET /games/:id ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct GameSeatRow {
    id:            Uuid,
    mode:          String,
    status:        String,
    created_by:    Option<Uuid>,
    created_at:    DateTime<Utc>,
    finished_at:   Option<DateTime<Utc>>,
    seat_index:    Option<i32>,
    seat_user_id:  Option<Uuid>,
    is_ai:         Option<bool>,
    ai_difficulty: Option<String>,
    is_winner:     Option<bool>,
    username:      Option<String>,
    display_name:  Option<String>,
    avatar_url:    Option<String>,
    is_guest:      Option<bool>,
}

#[derive(Serialize)]
pub struct SeatResponse {
    pub seat_index:    i32,
    pub is_ai:         bool,
    pub ai_difficulty: Option<String>,
    pub is_winner:     bool,
    pub user:          Option<PublicUser>,
}

#[derive(Serialize)]
pub struct GameResponse {
    pub id:          Uuid,
    pub mode:        String,
    pub status:      String,
    pub created_by:  Option<Uuid>,
    pub created_at:  DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub seats:       Vec<SeatResponse>,
}

pub async fn get_by_id(
    AuthUser(_claims): AuthUser,
    State(app): State<AppState>,
    Path(game_id): Path<Uuid>,
) -> Result<Json<GameResponse>, AppError> {
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
                seat_index:    idx,
                is_ai:         r.is_ai.unwrap_or(false),
                ai_difficulty: r.ai_difficulty.clone(),
                is_winner:     r.is_winner.unwrap_or(false),
                user:          r.seat_user_id.map(|uid| PublicUser {
                    id:           uid,
                    username:     r.username.clone().unwrap_or_default(),
                    display_name: r.display_name.clone().unwrap_or_default(),
                    avatar_url:   r.avatar_url.clone(),
                    is_guest:     r.is_guest.unwrap_or(false),
                }),
            })
        })
        .collect();

    Ok(Json(GameResponse {
        id:          first.id,
        mode:        first.mode.clone(),
        status:      first.status.clone(),
        created_by:  first.created_by,
        created_at:  first.created_at,
        finished_at: first.finished_at,
        seats,
    }))
}

// ── GameMode helper ────────────────────────────────────────────────────────────

impl GameMode {
    fn to_db_str(self) -> &'static str {
        match self {
            GameMode::Stack   => "stack",
            GameMode::NoStack => "no_stack",
        }
    }
}
