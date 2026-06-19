use api_core::response::ApiResponse;
use axum::extract::State;

use crate::context::AuthContext;
use crate::error::AuthError;
use crate::extractors::AuthUser;
use crate::services;

pub async fn list<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
) -> Result<ApiResponse, AuthError> {
    let sessions: Vec<crate::session::Session> = services::session_service::list_for_user(&ctx, &user.user_id).await?;
    Ok(ApiResponse::data("sessions", sessions))
}

pub async fn revoke<C: AuthContext>(
    State(ctx): State<C>,
    _user: AuthUser,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<ApiResponse, AuthError> {
    services::session_service::revoke(&ctx, &session_id).await?;
    Ok(ApiResponse::message("session revoked"))
}

pub async fn revoke_all<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
) -> Result<ApiResponse, AuthError> {
    services::session_service::revoke_all_for_user(&ctx, &user.user_id).await?;
    Ok(ApiResponse::message("all sessions revoked"))
}
