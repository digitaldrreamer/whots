use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::{user::PublicUser, User},
    state::AppState,
};

// ── GET /users/me ──────────────────────────────────────────────────────────────

pub async fn me(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<PublicUser>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(claims.sub)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))?;

    Ok(Json(user.into()))
}

// ── PUT /users/me ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Validate)]
pub struct UpdateMeRequest {
    #[validate(length(min = 1, max = 50))]
    pub display_name: Option<String>,
    pub avatar_url:   Option<String>,
}

pub async fn update_me(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<UpdateMeRequest>,
) -> Result<Json<PublicUser>, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let user = sqlx::query_as::<_, User>(
        "UPDATE users
         SET display_name = COALESCE($2, display_name),
             avatar_url   = COALESCE($3, avatar_url)
         WHERE id = $1
         RETURNING *",
    )
    .bind(claims.sub)
    .bind(body.display_name)
    .bind(body.avatar_url)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(user.into()))
}

// ── GET /users/search?q=<username> ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn search(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<PublicUser>>, AppError> {
    if params.q.len() < 2 {
        return Err(AppError::BadRequest("query must be at least 2 characters".into()));
    }

    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users
         WHERE username ILIKE $1
           AND is_guest = FALSE
         LIMIT 20",
    )
    .bind(format!("{}%", params.q))
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users.into_iter().map(Into::into).collect()))
}

// ── GET /users/:username ───────────────────────────────────────────────────────

pub async fn get_by_username(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<PublicUser>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))?;

    Ok(Json(user.into()))
}

// ── POST /users/contacts/upload ────────────────────────────────────────────────
// Deferred for PWA — native app will call this to upload hashed phone contacts.

#[derive(Deserialize)]
pub struct ContactsUpload {
    /// SHA-256 hashes of E.164-normalised phone numbers from device contacts
    pub hashes: Vec<String>,
}

pub async fn upload_contact_hashes(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<ContactsUpload>,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM contact_hashes WHERE user_id = $1")
        .bind(claims.sub)
        .execute(&state.db)
        .await?;

    for hash in body.hashes {
        sqlx::query(
            "INSERT INTO contact_hashes (user_id, contact_hash) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(claims.sub)
        .bind(hash)
        .execute(&state.db)
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── GET /users/me/games ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GameSummary {
    pub id:           Uuid,
    pub mode:         String,
    pub status:       String,
    pub created_at:   DateTime<Utc>,
    pub finished_at:  Option<DateTime<Utc>>,
    pub seat_index:   i32,
    pub is_winner:    bool,
    pub player_count: i64,
}

pub async fn my_games(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(pq): Query<PageQuery>,
) -> Result<Json<Vec<GameSummary>>, AppError> {
    let offset = pq.page.unwrap_or(0).max(0) * 20;
    let games = sqlx::query_as::<_, GameSummary>(
        "SELECT
             g.id, g.mode, g.status, g.created_at, g.finished_at,
             gs.seat_index, gs.is_winner,
             (SELECT COUNT(*)::BIGINT FROM game_seats gs2 WHERE gs2.game_id = g.id) AS player_count
         FROM game_seats gs
         JOIN games g ON g.id = gs.game_id
         WHERE gs.user_id = $1
         ORDER BY g.created_at DESC
         LIMIT 20 OFFSET $2",
    )
    .bind(claims.sub)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(games))
}

// ── GET /users/contacts/matches ────────────────────────────────────────────────
// Returns registered users who appear in BOTH users' contact lists (bidirectional).

pub async fn contact_matches(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<PublicUser>>, AppError> {
    let users = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         WHERE u.id <> $1
           AND u.phone_hash IS NOT NULL
           AND EXISTS (
               SELECT 1 FROM contact_hashes ch
               WHERE ch.user_id = $1 AND ch.contact_hash = u.phone_hash
           )
           AND EXISTS (
               SELECT 1 FROM contact_hashes ch2
               JOIN users me ON me.phone_hash = ch2.contact_hash
               WHERE ch2.user_id = u.id AND me.id = $1
           )
         ORDER BY u.username",
    )
    .bind(claims.sub)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(users.into_iter().map(Into::into).collect()))
}
