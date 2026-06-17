use axum::extract::State;
use axum::Json;

use api_core::response::ApiResponse;

use crate::auth::AuthUser;
use crate::app::services;
use crate::error::AppError;
use crate::state::AppState;
use crate::app::validators;
use crate::app::validators::auth::*;

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<ApiResponse, AppError> {
    let body = validators::validate(body)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    let tokens = services::auth_service::refresh(&state.config, &body.refresh_token).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "access_token": tokens.access_token,
        "refresh_token": tokens.refresh_token,
    })))
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<ApiResponse, AppError> {
    let body = validators::validate(body)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    let tokens = services::auth_service::register(
        &state.config,
        &body.email,
        &body.password,
        &body.name,
    )
    .await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "access_token": tokens.access_token,
        "refresh_token": tokens.refresh_token,
    })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<ApiResponse, AppError> {
    let body = validators::validate(body)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    let tokens = services::auth_service::login(&state.config, &body.email, &body.password).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "access_token": tokens.access_token,
        "refresh_token": tokens.refresh_token,
    })))
}

pub async fn logout(_user: AuthUser) -> ApiResponse {
    ApiResponse::ok(serde_json::json!({ "message": "logged out" }))
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> Result<ApiResponse, AppError> {
    let body = validators::validate(body)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    services::auth_service::forgot_password(&state.config, &state.mailer, &body.email).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "reset link sent" })))
}

pub async fn reset_password(
    Json(body): Json<ResetPasswordRequest>,
) -> Result<ApiResponse, AppError> {
    let body = validators::validate(body)
        .map_err(|e| AppError::BadRequest(e.to_string()))?;
    services::auth_service::reset_password(&body.token, &body.password).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "password reset" })))
}
