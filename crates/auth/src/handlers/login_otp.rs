use api_core::response::ApiResponse;
use axum::extract::State;
use axum::response::Response;

use crate::context::AuthContext;
use crate::error::AuthError;
use super::token_response;
use crate::services;
use crate::validators::ValidatedJson;
use crate::validators::{LoginOtpSendRequest, LoginOtpVerifyRequest};

pub async fn send<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<LoginOtpSendRequest>,
) -> Result<ApiResponse, AuthError> {
    services::login_otp_service::send_login_otp(&ctx, &body.email).await?;
    Ok(ApiResponse::message("login code sent"))
}

pub async fn verify<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<LoginOtpVerifyRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::login_otp_service::verify_login_otp(&ctx, &body.email, &body.code).await?;
    Ok(token_response(&ctx, &tokens, "authenticated via OTP"))
}
