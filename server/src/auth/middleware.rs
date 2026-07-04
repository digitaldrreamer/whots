use std::collections::HashMap;

use axum::{
    async_trait,
    extract::{FromRequestParts, Query},
    http::request::Parts,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

use crate::{auth::decode_token, error::AppError, state::AppState};

/// Extractor that validates the Bearer token and provides the JWT claims.
/// Use in route handlers: `async fn handler(AuthUser(claims): AuthUser, ...)`
pub struct AuthUser(pub crate::auth::Claims);

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::Unauthorized)?;

        let claims =
            decode_token(bearer.token(), &state.config).map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser(claims))
    }
}

/// Auth extractor for WebSocket upgrades. Browsers cannot set the
/// `Authorization` header on a `new WebSocket()` handshake, so this accepts the
/// token either from the `Authorization: Bearer` header (non-browser clients)
/// or from a `?token=<jwt>` query parameter (browsers).
pub struct WsAuthUser(pub crate::auth::Claims);

#[async_trait]
impl FromRequestParts<AppState> for WsAuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Prefer the standard header when present.
        if let Ok(TypedHeader(Authorization(bearer))) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await
        {
            if let Ok(claims) = decode_token(bearer.token(), &state.config) {
                return Ok(WsAuthUser(claims));
            }
        }

        // Fall back to the `token` query parameter.
        let Query(params) =
            Query::<HashMap<String, String>>::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::Unauthorized)?;
        let token = params.get("token").ok_or(AppError::Unauthorized)?;
        let claims = decode_token(token, &state.config).map_err(|_| AppError::Unauthorized)?;

        Ok(WsAuthUser(claims))
    }
}
