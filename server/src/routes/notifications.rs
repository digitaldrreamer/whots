use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tokio::sync::mpsc;

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

    sqlx::query("UPDATE notifications SET read = TRUE WHERE user_id = $1 AND NOT read")
        .bind(claims.sub)
        .execute(&app.db)
        .await?;

    Ok(Json(notifs))
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

// ── GET /ws/notify ─────────────────────────────────────────────────────────────

pub async fn notify_socket(
    ws: WebSocketUpgrade,
    AuthUser(claims): AuthUser,
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| handle_notify_socket(socket, claims.sub, app)))
}

async fn handle_notify_socket(mut socket: WebSocket, user_id: uuid::Uuid, app: AppState) {
    // Flush all unread notifications immediately
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

    loop {
        tokio::select! {
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
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    app.notify_txs.remove(&user_id);
}
