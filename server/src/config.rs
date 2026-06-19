use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry_seconds: i64,
    pub jwt_refresh_expiry_days: i64,
    pub port: u16,
    pub frontend_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: var("DATABASE_URL")?,
            jwt_secret: var("JWT_SECRET")?,
            jwt_access_expiry_seconds: var("JWT_ACCESS_EXPIRY_SECONDS")
                .unwrap_or_else(|_| "900".into())
                .parse()
                .context("JWT_ACCESS_EXPIRY_SECONDS must be a number")?,
            jwt_refresh_expiry_days: var("JWT_REFRESH_EXPIRY_DAYS")
                .unwrap_or_else(|_| "30".into())
                .parse()
                .context("JWT_REFRESH_EXPIRY_DAYS must be a number")?,
            port: var("PORT")
                .unwrap_or_else(|_| "3001".into())
                .parse()
                .context("PORT must be a number")?,
            frontend_url: var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
        })
    }
}

fn var(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("missing env var: {key}"))
}
