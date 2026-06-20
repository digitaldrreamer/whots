use std::{sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    models::notification::Notification,
    state::AppState,
};

// ── GET /notifications ─────────────────────────────────────────────────────────

pub async fn list(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<Json<Vec<Notification>>, AppError> {
    let notifs = sqlx::query_as::<_, Notification>(
        "SELECT * FROM notifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(claims.sub)
    .fetch_all(&app.db)
    .await?;
    Ok(Json(notifs))
}

// ── GET /notifications/count ───────────────────────────────────────────────────

pub async fn unread_count(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND NOT read")
            .bind(claims.sub)
            .fetch_one(&app.db)
            .await?;
    Ok(Json(json!({ "unread": count })))
}

// ── DELETE /notifications ──────────────────────────────────────────────────────

pub async fn mark_all_read(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<StatusCode, AppError> {
    sqlx::query("UPDATE notifications SET read = TRUE WHERE user_id = $1 AND NOT read")
        .bind(claims.sub)
        .execute(&app.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── PATCH /notifications/:id ───────────────────────────────────────────────────

pub async fn mark_one_read(
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let updated: Option<Uuid> = sqlx::query_scalar(
        "UPDATE notifications SET read = TRUE
         WHERE id = $1 AND user_id = $2
         RETURNING id",
    )
    .bind(id)
    .bind(claims.sub)
    .fetch_optional(&app.db)
    .await?;

    if updated.is_none() {
        return Err(AppError::NotFound("notification not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── GET /ws/notify ─────────────────────────────────────────────────────────────

pub async fn notify_socket(
    ws: WebSocketUpgrade,
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| handle_notify_socket(socket, claims.sub, app)))
}

async fn handle_notify_socket(mut socket: WebSocket, user_id: Uuid, app: AppState) {
    // Flush all unread notifications immediately on connect
    let unread = sqlx::query_as::<_, Notification>(
        "SELECT * FROM notifications WHERE user_id = $1 AND NOT read ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(&app.db)
    .await
    .unwrap_or_default();

    for notif in &unread {
        if let Ok(json) = serde_json::to_string(notif) {
            if socket.send(Message::Text(json.into())).await.is_err() {
                return;
            }
        }
    }
    if !unread.is_empty() {
        sqlx::query("UPDATE notifications SET read = TRUE WHERE user_id = $1 AND NOT read")
            .bind(user_id)
            .execute(&app.db)
            .await
            .ok();
    }

    // Register for live delivery
    let (tx, mut rx) = mpsc::unbounded_channel::<Arc<Notification>>();
    app.notify_txs.insert(user_id, tx);

    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_secs(30),
        Duration::from_secs(30),
    );

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(vec![])).await.is_err() { break; }
            }
            notif = rx.recv() => {
                match notif {
                    Some(n) => {
                        if let Ok(json) = serde_json::to_string(n.as_ref()) {
                            if socket.send(Message::Text(json.into())).await.is_err() { break; }
                        }
                    }
                    None => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    app.notify_txs.remove(&user_id);
}
