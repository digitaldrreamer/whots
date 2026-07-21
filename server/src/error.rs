use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("authentication required")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Conflict(String),

    /// A conflict the client needs structured detail about in order to offer a
    /// sensible next step — e.g. "you already have a game with this player",
    /// where the UI wants the game's id and start time to link to it.
    #[error("{message}")]
    ConflictWith {
        message: String,
        detail: serde_json::Value,
    },

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(e.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Carries its own body shape: `{ "error": ..., ...detail }`.
        if let AppError::ConflictWith { message, detail } = &self {
            let mut body = json!({ "error": message });
            if let (Some(obj), Some(extra)) = (body.as_object_mut(), detail.as_object()) {
                for (k, v) in extra {
                    obj.insert(k.clone(), v.clone());
                }
            }
            return (StatusCode::CONFLICT, Json(body)).into_response();
        }

        let (status, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            // Handled above — it builds its own response.
            AppError::ConflictWith { message, .. } => (StatusCode::CONFLICT, message.clone()),
            AppError::Internal(e) => {
                tracing::error!("internal error: {e:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".into(),
                )
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
