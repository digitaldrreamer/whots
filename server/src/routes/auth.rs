use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use validator::Validate;

use crate::{
    auth::{encode_access_token, hash_password, verify_password, AuthUser},
    error::AppError,
    models::{user::PublicUser, User},
    state::AppState,
};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn generate_refresh_token() -> (String, String) {
    let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 64);
    let hash  = hex::encode(Sha256::digest(token.as_bytes()));
    (token, hash)
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user:          PublicUser,
    pub access_token:  String,
    pub refresh_token: String,
}

async fn issue_tokens(user: User, state: &AppState) -> Result<AuthResponse, AppError> {
    let access_token = encode_access_token(&user, &state.config)
        .map_err(AppError::Internal)?;

    let (refresh_token, token_hash) = generate_refresh_token();
    let expires_at = Utc::now() + Duration::days(state.config.jwt_refresh_expiry_days);

    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    Ok(AuthResponse { user: user.into(), access_token, refresh_token })
}

// ── POST /auth/guest ───────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    static ref USERNAME_REGEX: regex::Regex =
        regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
}

#[derive(Deserialize, Validate)]
pub struct GuestRequest {
    #[validate(length(min = 3, max = 30), regex(path = *USERNAME_REGEX))]
    pub username: String,
}

pub async fn guest(
    State(state): State<AppState>,
    Json(body): Json<GuestRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let taken: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
            .bind(&body.username)
            .fetch_one(&state.db)
            .await?;

    if taken {
        return Err(AppError::Conflict("username already taken".into()));
    }

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, display_name, is_guest)
         VALUES ($1, $1, TRUE) RETURNING *",
    )
    .bind(&body.username)
    .fetch_one(&state.db)
    .await?;

    let tokens = issue_tokens(user, &state).await?;
    Ok((StatusCode::CREATED, Json(tokens)))
}

// ── POST /auth/register ────────────────────────────────────────────────────────

#[derive(Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 30), regex(path = *USERNAME_REGEX))]
    pub username: String,
    #[validate(email)]
    pub email:    String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let password_hash = hash_password(&body.password).map_err(AppError::Internal)?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, display_name, email, password_hash, is_guest)
         VALUES ($1, $1, $2, $3, FALSE) RETURNING *",
    )
    .bind(&body.username)
    .bind(body.email.to_lowercase())
    .bind(password_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::Conflict("username or email already registered".into())
        } else {
            AppError::Internal(e.into())
        }
    })?;

    let tokens = issue_tokens(user, &state).await?;
    Ok((StatusCode::CREATED, Json(tokens)))
}

// ── POST /auth/login ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginRequest {
    pub identifier: String, // email or username
    pub password:   String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let identifier = body.identifier.trim().to_lowercase();

    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users
         WHERE (email = $1 OR username = $1) AND is_guest = FALSE",
    )
    .bind(&identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let hash = user.password_hash.as_deref().ok_or(AppError::Unauthorized)?;

    if !verify_password(&body.password, hash).map_err(AppError::Internal)? {
        return Err(AppError::Unauthorized);
    }

    Ok(Json(issue_tokens(user, &state).await?))
}

// ── POST /auth/refresh ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let token_hash = hex::encode(Sha256::digest(body.refresh_token.as_bytes()));

    let row = sqlx::query_as::<_, (uuid::Uuid,)>(
        "DELETE FROM refresh_tokens
         WHERE token_hash = $1 AND expires_at > NOW()
         RETURNING user_id",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(row.0)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(issue_tokens(user, &state).await?))
}

// ── DELETE /auth/logout ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout(
    AuthUser(_claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<LogoutRequest>,
) -> Result<StatusCode, AppError> {
    let token_hash = hex::encode(Sha256::digest(body.refresh_token.as_bytes()));
    sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
