use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
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
