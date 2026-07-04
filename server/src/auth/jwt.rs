use anyhow::{Context, Result};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::Config, models::User};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid, // user id
    pub username: String,
    pub is_guest: bool,
    pub exp: i64, // unix timestamp
    pub iat: i64,
}

pub fn encode_access_token(user: &User, config: &Config) -> Result<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        is_guest: user.is_guest,
        iat: now,
        exp: now + config.jwt_access_expiry_seconds,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .context("failed to encode JWT")
}

pub fn decode_token(token: &str, config: &Config) -> Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .context("invalid token")?;
    Ok(data.claims)
}
