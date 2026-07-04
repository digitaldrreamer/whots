use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    game::{
        engine::create_game,
        types::{GameMode, Seat, SeatKind},
    },
    state::AppState,
    store::{game_store, matchmaking_store, notification_store},
};

// ── POST /matchmaking/join ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct JoinRequest {
    pub mode: GameMode,
}

#[derive(Serialize)]
pub struct JoinResponse {
    pub matched: bool,
    pub game_id: Option<Uuid>,
}

pub async fn join(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Json(body): Json<JoinRequest>,
) -> Result<Json<JoinResponse>, AppError> {
    let mut redis = app
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    // Remove caller from any existing queue first (mode switch / re-join).
    let _ = matchmaking_store::leave(&mut redis, claims.sub).await;

    // Try to find an opponent already waiting.
    if let Some(opponent_id) = matchmaking_store::pop_opponent(&mut redis, claims.sub, body.mode)
        .await
        .map_err(AppError::Internal)?
    {
        let opp_name: String = sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
            .bind(opponent_id)
            .fetch_optional(&app.db)
            .await?
            .unwrap_or_else(|| "Opponent".into());

        let seats = vec![
            Seat {
                name: claims.username.clone(),
                kind: SeatKind::Human {
                    user_id: claims.sub,
                },
                hand: vec![],
            },
            Seat {
                name: opp_name,
                kind: SeatKind::Human {
                    user_id: opponent_id,
                },
                hand: vec![],
            },
        ];
        let game_state = create_game(seats, body.mode);
        let game_id = game_state.id;

        sqlx::query(
            "INSERT INTO games (id, mode, status, created_by) VALUES ($1, $2, 'playing', $3)",
        )
        .bind(game_id)
        .bind(mode_str(body.mode))
        .bind(claims.sub)
        .execute(&app.db)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

        for (idx, uid) in [(0i32, claims.sub), (1, opponent_id)] {
            sqlx::query(
                "INSERT INTO game_seats (game_id, seat_index, user_id, is_ai, accepted_at)
                 VALUES ($1, $2, $3, FALSE, NOW())",
            )
            .bind(game_id)
            .bind(idx)
            .bind(uid)
            .execute(&app.db)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        }

        game_store::save(&mut redis, &game_state)
            .await
            .map_err(AppError::Internal)?;

        notification_store::push(
            &app.db,
            &app.notify_txs,
            opponent_id,
            "match_found",
            serde_json::json!({
                "game_id": game_id,
                "from_username": claims.username
            }),
        )
        .await;

        return Ok(Json(JoinResponse {
            matched: true,
            game_id: Some(game_id),
        }));
    }

    // No match yet — sit in queue.
    matchmaking_store::join(&mut redis, claims.sub, body.mode)
        .await
        .map_err(AppError::Internal)?;

    Ok(Json(JoinResponse {
        matched: false,
        game_id: None,
    }))
}

// ── DELETE /matchmaking/queue ─────────────────────────────────────────────────

pub async fn leave(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<StatusCode, AppError> {
    let mut redis = app
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    matchmaking_store::leave(&mut redis, claims.sub)
        .await
        .map_err(AppError::Internal)?;
    Ok(StatusCode::NO_CONTENT)
}

// ── GET /matchmaking/status ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct StatusResponse {
    pub in_queue: bool,
    pub mode: Option<String>,
}

pub async fn status(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<Json<StatusResponse>, AppError> {
    let mut redis = app
        .redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let mode = matchmaking_store::queued_mode(&mut redis, claims.sub)
        .await
        .map_err(AppError::Internal)?;
    Ok(Json(StatusResponse {
        in_queue: mode.is_some(),
        mode: mode.map(|m| mode_str(m).to_string()),
    }))
}

fn mode_str(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Stack => "stack",
        GameMode::NoStack => "no_stack",
    }
}
