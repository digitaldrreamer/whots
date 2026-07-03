use std::sync::Arc;

use axum::{body::Body, http::Request, response::Response, Router};
use dashmap::DashMap;
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use whots_server::{
    config::Config,
    make_router,
    state::AppState,
};

pub fn test_config() -> Config {
    Config {
        database_url:              std::env::var("DATABASE_URL").unwrap_or_default(),
        jwt_secret:                "test-secret-must-be-32-chars-or-more!!".into(),
        jwt_access_expiry_seconds: 900,
        jwt_refresh_expiry_days:   30,
        port:                      3001,
        frontend_url:              "http://localhost:5173".into(),
        redis_url:                 std::env::var("REDIS_URL")
                                       .unwrap_or_else(|_| "redis://127.0.0.1/".into()),
        app_url:                   "http://localhost:5173".into(),
        smtp_host:                 None,
        smtp_port:                 None,
        smtp_user:                 None,
        smtp_password:             None,
        smtp_from:                 None,
    }
}

pub fn make_app(pool: PgPool) -> Router {
    let config = test_config();
    let redis_url = config.redis_url.clone();
    let redis = redis::Client::open(redis_url.as_str()).expect("Redis client");
    let state = AppState {
        db:         pool,
        config:     Arc::new(config),
        redis,
        rooms:      Arc::new(DashMap::new()),
        notify_txs: Arc::new(DashMap::new()),
    };
    let origin = "http://localhost:5173".parse().unwrap();
    make_router(state, origin)
}

pub async fn req(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (axum::http::StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);

    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }

    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_string(&v).unwrap())
        }
        None => Body::empty(),
    };

    let response: Response = app.clone().oneshot(builder.body(body).unwrap()).await.unwrap();
    let status              = response.status();
    let bytes               = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

    (status, json)
}

/// Register a user and return (access_token, refresh_token).
pub async fn register_user(
    app: &Router,
    username: &str,
    password: &str,
) -> (String, String) {
    let (status, body) = req(
        app,
        "POST",
        "/api/auth/register",
        Some(serde_json::json!({
            "username": username,
            "display_name": username,
            "email": format!("{username}@test.example"),
            "password": password,
        })),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED, "register failed: {body}");
    (
        body["access_token"].as_str().unwrap().to_owned(),
        body["refresh_token"].as_str().unwrap().to_owned(),
    )
}

/// Create a guest and return access_token.
#[allow(dead_code)] // shared test helper, not used by every test binary
pub async fn guest_token(app: &Router, username: &str) -> String {
    let (status, body) = req(
        app,
        "POST",
        "/api/auth/guest",
        Some(serde_json::json!({ "username": username })),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED, "guest failed: {body}");
    body["access_token"].as_str().unwrap().to_owned()
}
