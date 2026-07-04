use axum::{
    extract::{Path, State},
    Json,
};
use rand::RngCore;
use serde::Serialize;
use uuid::Uuid;

use crate::{auth::AuthUser, error::AppError, state::AppState, store::notification_store};

fn gen_token() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    hex::encode(b) // 32 hex chars
}

// ── POST /invites ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CreateInviteResponse {
    pub token: String,
}

/// Mint a one-use friend-invite token for the caller.
pub async fn create(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<Json<CreateInviteResponse>, AppError> {
    let token = gen_token();
    sqlx::query("INSERT INTO friend_invites (token, creator_id) VALUES ($1, $2)")
        .bind(&token)
        .bind(claims.sub)
        .execute(&app.db)
        .await?;
    Ok(Json(CreateInviteResponse { token }))
}

// ── POST /invites/:token/redeem ─────────────────────────────────────────────

#[derive(Serialize)]
pub struct RedeemResponse {
    pub username: String,
    pub display_name: String,
}

/// Redeem an invite: the caller and the creator become friends immediately.
pub async fn redeem(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<RedeemResponse>, AppError> {
    let invite = sqlx::query_as::<_, (Uuid, Option<Uuid>, bool)>(
        "SELECT creator_id, used_by, (expires_at < NOW()) AS expired
         FROM friend_invites WHERE token = $1",
    )
    .bind(&token)
    .fetch_optional(&app.db)
    .await?
    .ok_or_else(|| AppError::NotFound("invite not found".into()))?;
    let (creator_id, used_by, expired) = invite;

    if used_by.is_some() {
        return Err(AppError::BadRequest("this invite has already been used".into()));
    }
    if expired {
        return Err(AppError::BadRequest("this invite has expired".into()));
    }
    if creator_id == claims.sub {
        return Err(AppError::BadRequest("you can't use your own invite".into()));
    }

    // Claim the token atomically (guards against a double-redeem race).
    let claimed = sqlx::query(
        "UPDATE friend_invites SET used_by = $1, used_at = NOW()
         WHERE token = $2 AND used_by IS NULL",
    )
    .bind(claims.sub)
    .bind(&token)
    .execute(&app.db)
    .await?;
    if claimed.rows_affected() == 0 {
        return Err(AppError::BadRequest("this invite has already been used".into()));
    }

    // Become friends (accept an existing edge in either direction, else insert).
    let accepted = sqlx::query(
        "UPDATE friends SET status = 'accepted', updated_at = NOW()
         WHERE (requester_id = $1 AND addressee_id = $2)
            OR (requester_id = $2 AND addressee_id = $1)",
    )
    .bind(creator_id)
    .bind(claims.sub)
    .execute(&app.db)
    .await?;
    if accepted.rows_affected() == 0 {
        sqlx::query(
            "INSERT INTO friends (requester_id, addressee_id, status)
             VALUES ($1, $2, 'accepted')",
        )
        .bind(creator_id)
        .bind(claims.sub)
        .execute(&app.db)
        .await?;
    }

    let (username, display_name): (String, String) =
        sqlx::query_as("SELECT username, display_name FROM users WHERE id = $1")
            .bind(creator_id)
            .fetch_one(&app.db)
            .await?;

    notification_store::push(
        &app.db,
        &app.notify_txs,
        creator_id,
        "friend_added",
        serde_json::json!({ "from_username": claims.username }),
    )
    .await;

    Ok(Json(RedeemResponse {
        username,
        display_name,
    }))
}
