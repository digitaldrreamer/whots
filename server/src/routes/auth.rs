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

fn dicebear_url(seed: &str) -> String {
    format!("https://api.dicebear.com/9.x/avataaars/svg?seed={seed}&backgroundColor=b6e3f4,c0aede,d1d4f9")
}

fn generate_token() -> (String, String) {
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

    let (refresh_token, token_hash) = generate_token();
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

    let avatar = dicebear_url(&body.username);
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, display_name, avatar_url, is_guest)
         VALUES ($1, $1, $2, TRUE) RETURNING *",
    )
    .bind(&body.username)
    .bind(avatar)
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

    let avatar = dicebear_url(&body.username);
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, display_name, email, password_hash, avatar_url, is_guest)
         VALUES ($1, $1, $2, $3, $4, FALSE) RETURNING *",
    )
    .bind(&body.username)
    .bind(body.email.to_lowercase())
    .bind(password_hash)
    .bind(avatar)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            AppError::Conflict("username or email already registered".into())
        } else {
            AppError::Internal(e.into())
        }
    })?;

    // Fire-and-forget email verification
    send_verification_for(&user, &state).await;

    let tokens = issue_tokens(user, &state).await?;
    Ok((StatusCode::CREATED, Json(tokens)))
}

async fn send_verification_for(user: &User, state: &AppState) {
    let Some(ref email) = user.email else { return };
    let (token, hash) = generate_token();
    let expires_at = Utc::now() + Duration::hours(24);

    if let Err(e) = sqlx::query(
        "INSERT INTO email_verifications (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user.id)
    .bind(&hash)
    .bind(expires_at)
    .execute(&state.db)
    .await
    {
        tracing::warn!(user_id = %user.id, "could not insert email verification: {e}");
        return;
    }

    if let Err(e) = crate::mail::send_verification(&state.config, email, &token).await {
        tracing::warn!("failed to send verification email to {email}: {e}");
    }
}

// ── POST /auth/login ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginRequest {
    pub identifier: String,
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

// ── POST /auth/forgot-password ─────────────────────────────────────────────────

#[derive(Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> Result<StatusCode, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let email = body.email.trim().to_lowercase();

    // Silently succeed whether or not the email exists (don't leak account existence)
    if let Some(user) = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1 AND is_guest = FALSE",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await?
    {
        let (token, hash) = generate_token();
        let expires_at = Utc::now() + Duration::hours(1);

        // Invalidate any existing reset tokens for this user
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
            .bind(user.id)
            .execute(&state.db)
            .await?;

        sqlx::query(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
             VALUES ($1, $2, $3)",
        )
        .bind(user.id)
        .bind(&hash)
        .bind(expires_at)
        .execute(&state.db)
        .await?;

        if let Err(e) = crate::mail::send_password_reset(&state.config, &email, &token).await {
            tracing::warn!("failed to send reset email to {email}: {e}");
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /auth/reset-password ──────────────────────────────────────────────────

#[derive(Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token:        String,
    #[validate(length(min = 8, max = 128))]
    pub new_password: String,
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(body): Json<ResetPasswordRequest>,
) -> Result<StatusCode, AppError> {
    body.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let hash = hex::encode(Sha256::digest(body.token.as_bytes()));

    let row = sqlx::query_as::<_, (uuid::Uuid,)>(
        "UPDATE password_reset_tokens SET used = TRUE
         WHERE token_hash = $1 AND expires_at > NOW() AND NOT used
         RETURNING user_id",
    )
    .bind(&hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("invalid or expired reset token".into()))?;

    let password_hash = hash_password(&body.new_password).map_err(AppError::Internal)?;

    sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
        .bind(row.0)
        .bind(password_hash)
        .execute(&state.db)
        .await?;

    // Invalidate all sessions after password change
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(row.0)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /auth/verify-email ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Json(body): Json<VerifyEmailRequest>,
) -> Result<StatusCode, AppError> {
    let hash = hex::encode(Sha256::digest(body.token.as_bytes()));

    let row = sqlx::query_as::<_, (uuid::Uuid,)>(
        "DELETE FROM email_verifications
         WHERE token_hash = $1 AND expires_at > NOW()
         RETURNING user_id",
    )
    .bind(&hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("invalid or expired verification token".into()))?;

    sqlx::query("UPDATE users SET email_verified = TRUE WHERE id = $1")
        .bind(row.0)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /auth/resend-verification ────────────────────────────────────────────

pub async fn resend_verification(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    if claims.is_guest {
        return Err(AppError::BadRequest("guests cannot verify email".into()));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(claims.sub)
        .fetch_one(&state.db)
        .await?;

    if user.email_verified {
        return Err(AppError::BadRequest("email already verified".into()));
    }

    // Delete existing, insert fresh
    sqlx::query("DELETE FROM email_verifications WHERE user_id = $1")
        .bind(claims.sub)
        .execute(&state.db)
        .await?;

    send_verification_for(&user, &state).await;
    Ok(StatusCode::NO_CONTENT)
}
