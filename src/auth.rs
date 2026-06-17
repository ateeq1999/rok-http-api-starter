use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use api_core::auth::Claims;
use api_core::response::{ApiResponse, ErrorCode};

use crate::state::AppState;

/// Extractor that reads JWT claims from request extensions.
/// Requires `JwtAuthLayer` middleware to have run first.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub claims: Claims,
    pub user_id: String,
    pub roles: Vec<String>,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(AuthRejection::MissingToken)?;

        let user_id = claims.sub.clone();
        let roles: Vec<String> = claims.roles.split(',').map(|s| s.trim().to_string()).collect();

        Ok(AuthUser {
            claims,
            user_id,
            roles,
        })
    }
}

/// Extractor that requires admin role.
#[allow(dead_code)]
pub struct AdminOnly(pub AuthUser);

impl<S> FromRequestParts<S> for AdminOnly
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.roles.iter().any(|r| r == "admin") {
            return Err(AuthRejection::Forbidden);
        }
        Ok(AdminOnly(user))
    }
}

#[derive(Debug)]
pub enum AuthRejection {
    MissingToken,
    Forbidden,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
            Self::MissingToken => (ErrorCode::Unauthorized, "unauthenticated"),
            Self::Forbidden => (ErrorCode::Forbidden, "insufficient permissions"),
        };
        ApiResponse::error(code, msg).into_response()
    }
}
