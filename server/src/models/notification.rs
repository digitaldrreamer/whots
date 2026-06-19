use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub kind:       String,
    pub payload:    serde_json::Value,
    pub read:       bool,
    pub created_at: DateTime<Utc>,
}
