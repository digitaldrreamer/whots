use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub db: bool,
}

pub async fn health(State(app): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db = sqlx::query("SELECT 1").execute(&app.db).await.is_ok();
    let code = if db {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        code,
        Json(HealthResponse {
            status: if db { "ok" } else { "degraded" },
            db,
        }),
    )
}
