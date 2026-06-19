use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url:              String,
    pub jwt_secret:                String,
    pub jwt_access_expiry_seconds: i64,
    pub jwt_refresh_expiry_days:   i64,
    pub port:                      u16,
    pub frontend_url:              String,
    pub redis_url:                 String,
    pub app_url:                   String,
    // SMTP — all optional; if smtp_host is None, emails are logged to stdout (dev mode)
    pub smtp_host:     Option<String>,
    pub smtp_port:     Option<u16>,
    pub smtp_user:     Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from:     Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: var("DATABASE_URL")?,
            jwt_secret:   var("JWT_SECRET")?,
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
            redis_url: var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1/".into()),
            app_url: var("APP_URL")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
            smtp_host:     std::env::var("SMTP_HOST").ok(),
            smtp_port:     std::env::var("SMTP_PORT").ok().and_then(|v| v.parse().ok()),
            smtp_user:     std::env::var("SMTP_USER").ok(),
            smtp_password: std::env::var("SMTP_PASSWORD").ok(),
            smtp_from:     std::env::var("SMTP_FROM").ok(),
        })
    }
}

fn var(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("missing env var: {key}"))
}
