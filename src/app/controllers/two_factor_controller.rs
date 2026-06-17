use api_core::response::ApiResponse;

use auth::extractors::AuthUser;
use crate::app::services;
use crate::error::AppError;
use auth::validators::ValidatedJson;
use auth::validators::{TwoFactorDisableRequest, TwoFactorVerifyRequest};

pub async fn enable(user: AuthUser) -> Result<ApiResponse, AppError> {
    let result = services::two_factor_service::enable(&user.user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "secret": result.secret,
        "otpauth_url": result.otpauth_url,
        "backup_codes": result.backup_codes,
    })))
}

pub async fn verify(
    user: AuthUser,
    ValidatedJson(body): ValidatedJson<TwoFactorVerifyRequest>,
) -> Result<ApiResponse, AppError> {
    services::two_factor_service::verify_and_activate(&user.user_id, &body.code).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "2FA enabled" })))
}

pub async fn disable(
    user: AuthUser,
    ValidatedJson(body): ValidatedJson<TwoFactorDisableRequest>,
) -> Result<ApiResponse, AppError> {
    services::two_factor_service::disable(&user.user_id, &body.password, &body.code).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "2FA disabled" })))
}
