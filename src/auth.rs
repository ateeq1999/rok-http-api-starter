use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use api_core::auth::{verify_token, Claims};
use api_core::response::{ApiResponse, ErrorCode};

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct AuthUser {
    #[allow(dead_code)]
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

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthRejection::MissingToken)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AuthRejection::InvalidScheme)?;

        let claims = verify_token(token, &app_state.config.auth_secret)
            .map_err(|_| AuthRejection::InvalidToken)?;

        let user_id = claims.sub.clone();
        let roles: Vec<String> = claims.roles.split(',').map(|s| s.trim().to_string()).collect();

        Ok(AuthUser {
            claims,
            user_id,
            roles,
        })
    }
}

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
    InvalidScheme,
    InvalidToken,
    Forbidden,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let (code, msg) = match self {
            Self::MissingToken => (ErrorCode::Unauthorized, "missing authorization header"),
            Self::InvalidScheme => (ErrorCode::Unauthorized, "invalid authorization scheme, use Bearer"),
            Self::InvalidToken => (ErrorCode::Unauthorized, "invalid or expired token"),
            Self::Forbidden => (ErrorCode::Forbidden, "insufficient permissions"),
        };
        ApiResponse::error(code, msg).into_response()
    }
}
