use axum::extract::State;

use api_core::response::ApiResponse;

use crate::error::AppError;
use crate::app::services;
use crate::state::AppState;
use crate::app::validators::ValidatedJson;
use crate::app::validators::otp::*;

pub async fn send(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<SendOtpRequest>,
) -> Result<ApiResponse, AppError> {
    services::otp_service::send(&state.config, &state.mailer, &body.email).await?;
    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "verification email sent" }),
    ))
}

pub async fn verify(
    ValidatedJson(body): ValidatedJson<VerifyOtpRequest>,
) -> Result<ApiResponse, AppError> {
    services::otp_service::verify(&body.email, &body.code).await?;
    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "email verified" }),
    ))
}
