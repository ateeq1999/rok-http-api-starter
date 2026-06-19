use api_core::response::ApiResponse;
use axum::extract::State;

use crate::context::AuthContext;
use crate::error::AuthError;
use crate::services;
use crate::validators::ValidatedJson;
use crate::validators::{SendOtpRequest, VerifyOtpRequest};

pub async fn send<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<SendOtpRequest>,
) -> Result<ApiResponse, AuthError> {
    services::otp_service::send(&ctx, &body.email).await?;
    Ok(ApiResponse::message("verification email sent"))
}

pub async fn verify<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<VerifyOtpRequest>,
) -> Result<ApiResponse, AuthError> {
    services::otp_service::verify(&ctx, &body.email, &body.code).await?;
    Ok(ApiResponse::message("email verified"))
}
