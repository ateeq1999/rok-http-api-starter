use axum::extract::State;

use api_core::response::ApiResponse;

use auth::extractors::AuthUser;
use crate::app::services;
use crate::error::AppError;
use crate::state::AppState;
use auth::validators::ValidatedJson;
use auth::validators::{RefreshRequest, RegisterRequest, LoginRequest, ForgotPasswordRequest, ResetPasswordRequest};

pub async fn refresh(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RefreshRequest>,
) -> Result<ApiResponse, AppError> {
    let tokens = services::auth_service::refresh(&state.config, &body.refresh_token).await?;
    Ok(ApiResponse::ok(serde_json::json!(tokens)))
}

pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> Result<ApiResponse, AppError> {
    let tokens = services::auth_service::register(
        &state.config,
        &body.email,
        &body.password,
        &body.name,
    )
    .await?;
    Ok(ApiResponse::ok(serde_json::json!(tokens)))
}

pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> Result<ApiResponse, AppError> {
    let tokens = services::auth_service::login(&state.config, &body.email, &body.password).await?;
    Ok(ApiResponse::ok(serde_json::json!(tokens)))
}

pub async fn logout(_user: AuthUser) -> ApiResponse {
    ApiResponse::ok(serde_json::json!({ "message": "logged out" }))
}

pub async fn forgot_password(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<ForgotPasswordRequest>,
) -> Result<ApiResponse, AppError> {
    services::auth_service::forgot_password(&state.config, &state.mailer, &body.email).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "reset link sent" })))
}

pub async fn reset_password(
    ValidatedJson(body): ValidatedJson<ResetPasswordRequest>,
) -> Result<ApiResponse, AppError> {
    services::auth_service::reset_password(&body.token, &body.password).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "password reset" })))
}
