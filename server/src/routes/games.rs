use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    game::{
        engine::create_game,
        types::{Difficulty, GameMode, Seat, SeatKind},
    },
    state::AppState,
    store::game_store,
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

    // Caller must be one of the human seats
    let caller_present = body.seats.iter().any(|s| {
        matches!(s, SeatSpec::Human { user_id, .. } if *user_id == claims.sub)
    });
    if !caller_present {
        return Err(AppError::BadRequest("you must be a participant in the game you create".into()));
    }

    let seats: Vec<Seat> = body
        .seats
        .into_iter()
        .map(|spec| match spec {
            SeatSpec::Human { user_id, name } => Seat {
                name,
                kind: SeatKind::Human { user_id },
                hand: vec![],
            },
            SeatSpec::Ai { difficulty, name } => Seat {
                name,
                kind: SeatKind::Ai { difficulty },
                hand: vec![],
            },
        })
        .collect();

    let game_state = create_game(seats, body.mode);
    let game_id = game_state.id;

    // Record in Postgres (lightweight row — just the id and metadata)
    sqlx::query(
        "INSERT INTO games (id, mode, status, created_by) VALUES ($1, $2, 'playing', $3)",
    )
    .bind(game_id)
    .bind(body.mode.to_db_str())
    .bind(claims.sub)
    .execute(&app.db)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    // Save full game state to Redis
    let mut redis = app
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    game_store::save(&mut redis, &game_state)
        .await
        .map_err(|e| AppError::Internal(e))?;

    Ok((StatusCode::CREATED, Json(CreateGameResponse { game_id })))
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
