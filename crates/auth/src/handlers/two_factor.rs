use api_core::response::ApiResponse;
use axum::extract::State;

use crate::context::AuthContext;
use crate::error::AuthError;
use crate::extractors::AuthUser;
use crate::services;
use crate::validators::ValidatedJson;
use crate::validators::{TwoFactorDisableRequest, TwoFactorVerifyRequest};

pub async fn enable<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
) -> Result<ApiResponse, AuthError> {
    let result = services::two_factor_service::enable(&ctx, &user.user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "secret": result.secret,
        "otpauth_url": result.otpauth_url,
        "backup_codes": result.backup_codes,
    })))
}

pub async fn verify<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
    ValidatedJson(body): ValidatedJson<TwoFactorVerifyRequest>,
) -> Result<ApiResponse, AuthError> {
    services::two_factor_service::verify_and_activate(&ctx, &user.user_id, &body.code).await?;
    Ok(ApiResponse::message("2FA enabled"))
}

pub async fn disable<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
    ValidatedJson(body): ValidatedJson<TwoFactorDisableRequest>,
) -> Result<ApiResponse, AuthError> {
    services::two_factor_service::disable(&ctx, &user.user_id, &body.password, &body.code).await?;
    Ok(ApiResponse::message("2FA disabled"))
}
