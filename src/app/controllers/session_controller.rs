use axum::extract::{Path, State};

use api_core::response::ApiResponse;

use auth::extractors::AuthUser;
use crate::app::services;
use crate::error::AppError;
use crate::state::AppState;

pub async fn list(user: AuthUser) -> Result<ApiResponse, AppError> {
    let sessions = services::session_service::list_for_user(&user.user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "sessions": sessions })))
}

pub async fn revoke(
    user: AuthUser,
    Path(session_id): Path<String>,
) -> Result<ApiResponse, AppError> {
    // Verify the session belongs to this user
    let sessions = services::session_service::list_for_user(&user.user_id).await?;
    if !sessions.iter().any(|s| s.id == session_id) {
        return Err(AppError::NotFound("session not found".into()));
    }
    services::session_service::revoke(&session_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "session revoked" })))
}

pub async fn revoke_all(
    _state: State<AppState>,
    user: AuthUser,
) -> Result<ApiResponse, AppError> {
    services::session_service::revoke_all_for_user(&user.user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "all sessions revoked" })))
}
