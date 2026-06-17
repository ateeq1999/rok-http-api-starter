use axum::extract::State;
use axum::Json;

use api_core::response::ApiResponse;

use crate::error::AppError;
use crate::app::services;
use crate::state::AppState;
use crate::app::validators;
use crate::app::validators::otp::*;

pub async fn send(
    State(state): State<AppState>,
    Json(body): Json<SendOtpRequest>,
) -> Result<ApiResponse, AppError> {
    let body = validators::validate(body)
        .map_err(|_| AppError::BadRequest("invalid request".into()))?;
    services::otp_service::send(&state.config, &state.mailer, &body.email).await?;
    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "verification email sent" }),
    ))
}

pub async fn verify(
    Json(body): Json<VerifyOtpRequest>,
) -> Result<ApiResponse, AppError> {
    let body = validators::validate(body)
        .map_err(|_| AppError::BadRequest("invalid request".into()))?;
    services::otp_service::verify(&body.email, &body.code).await?;
    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "email verified" }),
    ))
}
