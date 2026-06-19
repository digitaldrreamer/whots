use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{auth::AuthUser, error::AppError, models::user::PublicUser, models::User, state::AppState};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FriendRow {
    pub id:           Uuid,
    pub username:     String,
    pub display_name: String,
    pub avatar_url:   Option<String>,
    pub since:        DateTime<Utc>,
}

// ── GET /friends ───────────────────────────────────────────────────────────────

pub async fn list(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<FriendRow>>, AppError> {
    let friends = sqlx::query_as::<_, FriendRow>(
        "SELECT
             u.id, u.username, u.display_name, u.avatar_url,
             f.updated_at AS since
         FROM friends f
         JOIN users u ON (
             CASE WHEN f.requester_id = $1 THEN f.addressee_id ELSE f.requester_id END = u.id
         )
         WHERE (f.requester_id = $1 OR f.addressee_id = $1)
           AND f.status = 'accepted'
         ORDER BY u.username",
    )
    .bind(claims.sub)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(friends))
}

// ── GET /friends/requests ──────────────────────────────────────────────────────

pub async fn incoming_requests(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<PublicUser>>, AppError> {
    let users = sqlx::query_as::<_, User>(
        "SELECT u.* FROM friends f
         JOIN users u ON u.id = f.requester_id
         WHERE f.addressee_id = $1 AND f.status = 'pending'
         ORDER BY f.created_at DESC",
    )
    .bind(claims.sub)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users.into_iter().map(Into::into).collect()))
}

// ── POST /friends/request/:username ───────────────────────────────────────────

pub async fn send_request(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<StatusCode, AppError> {
    let addressee = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))?;

    if addressee.id == claims.sub {
        return Err(AppError::BadRequest("cannot add yourself".into()));
    }

    sqlx::query(
        "INSERT INTO friends (requester_id, addressee_id, status)
         VALUES ($1, $2, 'pending')
         ON CONFLICT (requester_id, addressee_id) DO NOTHING",
    )
    .bind(claims.sub)
    .bind(addressee.id)
    .execute(&state.db)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /friends/request/:username/accept ─────────────────────────────────────

pub async fn accept_request(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<StatusCode, AppError> {
    let requester = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))?;

    let updated = sqlx::query(
        "UPDATE friends SET status = 'accepted'
         WHERE requester_id = $1 AND addressee_id = $2 AND status = 'pending'",
    )
    .bind(requester.id)
    .bind(claims.sub)
    .execute(&state.db)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("no pending request from that user".into()));
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /friends/request/:username/decline ────────────────────────────────────

pub async fn decline_request(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<StatusCode, AppError> {
    let requester = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))?;

    sqlx::query("DELETE FROM friends WHERE requester_id = $1 AND addressee_id = $2")
        .bind(requester.id)
        .bind(claims.sub)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── DELETE /friends/:username ──────────────────────────────────────────────────

pub async fn remove(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<StatusCode, AppError> {
    let other = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))?;

    sqlx::query(
        "DELETE FROM friends
         WHERE (requester_id = $1 AND addressee_id = $2)
            OR (requester_id = $2 AND addressee_id = $1)",
    )
    .bind(claims.sub)
    .bind(other.id)
    .execute(&state.db)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
