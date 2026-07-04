use axum::{extract::State, http::StatusCode, Json};
use redis::AsyncCommands;
use serde::Deserialize;
use webauthn_rs::prelude::*;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::User,
    routes::auth::{issue_tokens, AuthResponse},
    state::AppState,
};

const STATE_TTL: u64 = 300; // 5-min challenge window

async fn redis_conn(app: &AppState) -> Result<redis::aio::MultiplexedConnection, AppError> {
    app.redis
        .get_multiplexed_tokio_connection()
        .await
        .map_err(|e| AppError::Internal(e.into()))
}

fn wa_err(e: WebauthnError) -> AppError {
    AppError::BadRequest(format!("passkey error: {e}"))
}

async fn user_passkeys(app: &AppState, user_id: uuid::Uuid) -> Result<Vec<Passkey>, AppError> {
    let rows: Vec<serde_json::Value> =
        sqlx::query_scalar("SELECT credential FROM passkeys WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(&app.db)
            .await?;
    Ok(rows
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Passkey>(v).ok())
        .collect())
}

// ── POST /auth/passkey/register/start (authed) ──────────────────────────────

pub async fn register_start(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<Json<CreationChallengeResponse>, AppError> {
    let exclude: Vec<CredentialID> = user_passkeys(&app, claims.sub)
        .await?
        .iter()
        .map(|pk| pk.cred_id().clone())
        .collect();

    let (ccr, reg_state) = app
        .webauthn
        .start_passkey_registration(
            claims.sub,
            &claims.username,
            &claims.username,
            (!exclude.is_empty()).then_some(exclude),
        )
        .map_err(wa_err)?;

    let mut conn = redis_conn(&app).await?;
    let json = serde_json::to_string(&reg_state).map_err(|e| AppError::Internal(e.into()))?;
    conn.set_ex::<_, _, ()>(format!("pkreg:{}", claims.sub), json, STATE_TTL)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(ccr))
}

// ── POST /auth/passkey/register/finish (authed) ─────────────────────────────

pub async fn register_finish(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Json(reg): Json<RegisterPublicKeyCredential>,
) -> Result<StatusCode, AppError> {
    let mut conn = redis_conn(&app).await?;
    let key = format!("pkreg:{}", claims.sub);
    let raw: Option<String> = conn.get(&key).await.map_err(|e| AppError::Internal(e.into()))?;
    let reg_state: PasskeyRegistration = serde_json::from_str(
        &raw.ok_or_else(|| AppError::BadRequest("no passkey registration in progress".into()))?,
    )
    .map_err(|e| AppError::Internal(e.into()))?;

    let passkey = app
        .webauthn
        .finish_passkey_registration(&reg, &reg_state)
        .map_err(wa_err)?;
    let _: Result<(), _> = conn.del(&key).await;

    let cred_id = passkey.cred_id().as_ref().to_vec();
    let json = serde_json::to_value(&passkey).map_err(|e| AppError::Internal(e.into()))?;
    sqlx::query(
        "INSERT INTO passkeys (credential_id, user_id, credential) VALUES ($1, $2, $3)
         ON CONFLICT (credential_id) DO NOTHING",
    )
    .bind(&cred_id)
    .bind(claims.sub)
    .bind(&json)
    .execute(&app.db)
    .await?;

    // Adding a passkey claims the account (email-free) — it's no longer a guest.
    sqlx::query("UPDATE users SET is_guest = FALSE WHERE id = $1")
        .bind(claims.sub)
        .execute(&app.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ── POST /auth/passkey/login/start ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginStartRequest {
    pub username: String,
}

pub async fn login_start(
    State(app): State<AppState>,
    Json(body): Json<LoginStartRequest>,
) -> Result<Json<RequestChallengeResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&body.username)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(|| AppError::BadRequest("no passkey for that account".into()))?;

    let creds = user_passkeys(&app, user.id).await?;
    if creds.is_empty() {
        return Err(AppError::BadRequest("no passkey for that account".into()));
    }

    let (rcr, auth_state) = app
        .webauthn
        .start_passkey_authentication(&creds)
        .map_err(wa_err)?;

    let mut conn = redis_conn(&app).await?;
    let json = serde_json::to_string(&auth_state).map_err(|e| AppError::Internal(e.into()))?;
    conn.set_ex::<_, _, ()>(format!("pkauth:{}", user.id), json, STATE_TTL)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    Ok(Json(rcr))
}

// ── POST /auth/passkey/login/finish ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginFinishRequest {
    pub username: String,
    pub credential: PublicKeyCredential,
}

pub async fn login_finish(
    State(app): State<AppState>,
    Json(body): Json<LoginFinishRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&body.username)
        .fetch_optional(&app.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let mut conn = redis_conn(&app).await?;
    let key = format!("pkauth:{}", user.id);
    let raw: Option<String> = conn.get(&key).await.map_err(|e| AppError::Internal(e.into()))?;
    let auth_state: PasskeyAuthentication = serde_json::from_str(
        &raw.ok_or_else(|| AppError::BadRequest("no passkey login in progress".into()))?,
    )
    .map_err(|e| AppError::Internal(e.into()))?;

    let result = app
        .webauthn
        .finish_passkey_authentication(&body.credential, &auth_state)
        .map_err(|_| AppError::Unauthorized)?;
    let _: Result<(), _> = conn.del(&key).await;

    // Bump the stored signature counter if the authenticator advanced it.
    if result.needs_update() {
        let cred_id = result.cred_id().as_ref().to_vec();
        if let Some(v) = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT credential FROM passkeys WHERE credential_id = $1",
        )
        .bind(&cred_id)
        .fetch_optional(&app.db)
        .await?
        {
            if let Ok(mut pk) = serde_json::from_value::<Passkey>(v) {
                pk.update_credential(&result);
                let updated = serde_json::to_value(&pk).map_err(|e| AppError::Internal(e.into()))?;
                sqlx::query("UPDATE passkeys SET credential = $2 WHERE credential_id = $1")
                    .bind(&cred_id)
                    .bind(&updated)
                    .execute(&app.db)
                    .await?;
            }
        }
    }

    let tokens = issue_tokens(user, &app).await?;
    Ok(Json(tokens))
}
