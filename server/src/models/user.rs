use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id:            Uuid,
    pub username:      String,
    pub display_name:  String,
    pub email:         Option<String>,
    pub phone_hash:    Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub avatar_url:    Option<String>,
    pub is_guest:      bool,
    pub created_at:    DateTime<Utc>,
    pub updated_at:    DateTime<Utc>,
}

/// What we send back to the client — no password hash, no phone hash.
#[derive(Debug, Serialize)]
pub struct PublicUser {
    pub id:           Uuid,
    pub username:     String,
    pub display_name: String,
    pub avatar_url:   Option<String>,
    pub is_guest:     bool,
}

impl From<User> for PublicUser {
    fn from(u: User) -> Self {
        Self {
            id:           u.id,
            username:     u.username,
            display_name: u.display_name,
            avatar_url:   u.avatar_url,
            is_guest:     u.is_guest,
        }
    }
}
