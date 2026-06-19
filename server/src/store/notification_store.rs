use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::models::notification::Notification;

pub type NotifyTxMap = DashMap<Uuid, mpsc::UnboundedSender<Arc<Notification>>>;

pub async fn push(
    db: &sqlx::PgPool,
    notify_txs: &NotifyTxMap,
    user_id: Uuid,
    kind: &str,
    payload: Value,
) {
    let result = sqlx::query_as::<_, Notification>(
        "INSERT INTO notifications (user_id, kind, payload) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(user_id)
    .bind(kind)
    .bind(&payload)
    .fetch_one(db)
    .await;

    match result {
        Ok(notif) => {
            if let Some(tx) = notify_txs.get(&user_id) {
                let _ = tx.send(Arc::new(notif));
            }
        }
        Err(e) => tracing::warn!(%user_id, "notification persist failed: {e}"),
    }
}
