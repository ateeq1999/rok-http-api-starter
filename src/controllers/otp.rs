use rok_auth::axum::GuestOnly;
use rok_auth::axum::RequestContext;
use rok_core::api::ApiResponse;
use rok_validate::Valid;

use crate::models::User;
use crate::validators::otp::*;

pub async fn send(
    ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<SendOtpRequest>,
) -> ApiResponse {
    let user = match User::find_by_email(&body.email).await {
        Err(e) => return ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => return ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(u)) => u,
    };

    match rok_auth::EmailVerification::issue(ctx.db(), user.id).await {
        Err(e) => ApiResponse::error("E_OTP_SEND", e.to_string(), 500),
        Ok(_) => ApiResponse::ok(serde_json::json!({ "message": "verification email sent" })),
    }
}

pub async fn verify(
    ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<VerifyOtpRequest>,
) -> ApiResponse {
    let user = match User::find_by_email(&body.email).await {
        Err(e) => return ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => return ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(u)) => u,
    };

    let token = format!("{}{}", user.id, body.code);

    match rok_auth::EmailVerification::verify(ctx.db(), &token).await {
        Err(e) => ApiResponse::error("E_OTP_VERIFY", e.to_string(), 400),
        Ok(_) => ApiResponse::ok(serde_json::json!({ "message": "email verified" })),
    }
}
