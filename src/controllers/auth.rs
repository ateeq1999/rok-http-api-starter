use rok_auth::axum::GuestOnly;
use rok_auth::axum::RequestContext;
use rok_auth::{login, register as register_macro, password, AuthError, Claims};
use rok_core::api::ApiResponse;
use rok_orm::PgModel;
use rok_validate::Valid;

use crate::models::User;
use crate::validators::auth::*;

pub async fn register(
    ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<RegisterRequest>,
) -> ApiResponse {
    match User::find_by_email(&body.email).await {
        Ok(Some(_)) => return ApiResponse::error("E_DUPLICATE_EMAIL", "email already taken", 409),
        Err(e) => return ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => {}
    }

    let pool = ctx.db().clone();
    if let Err(e) = register_macro!(&body.email, &body.password, |email: String, hash: String| async move {
        User::create_user(&pool, &email, &hash, &body.name)
            .await
            .map(|_| ())
            .map_err(|e| AuthError::Internal(e.to_string()))
    }) {
        return ApiResponse::error("E_REGISTRATION", e.to_string(), 500);
    }

    match login!(&*ctx.auth, ctx.db(), &body.email, &body.password, User) {
        Err(e) => ApiResponse::error("E_LOGIN", e.to_string(), 401),
        Ok(tokens) => ApiResponse::ok(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
        })),
    }
}

pub async fn login(
    ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<LoginRequest>,
) -> ApiResponse {
    match login!(&*ctx.auth, ctx.db(), &body.email, &body.password, User) {
        Err(e) => ApiResponse::error("E_LOGIN", e.to_string(), 401),
        Ok(tokens) => ApiResponse::ok(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
        })),
    }
}

pub async fn logout(_ctx: RequestContext, _claims: Claims) -> ApiResponse {
    ApiResponse::ok(serde_json::json!({ "message": "logged out" }))
}

pub async fn forgot_password(
    ctx: RequestContext,
    Valid(body): Valid<ForgotPasswordRequest>,
) -> ApiResponse {
    match rok_auth::PasswordReset::issue(ctx.db(), &body.email).await {
        Err(e) => ApiResponse::error("E_RESET_ISSUE", e.to_string(), 500),
        Ok(_) => ApiResponse::ok(serde_json::json!({ "message": "reset link sent" })),
    }
}

pub async fn reset_password(
    ctx: RequestContext,
    Valid(body): Valid<ResetPasswordRequest>,
) -> ApiResponse {
    let email = match rok_auth::PasswordReset::verify(ctx.db(), &body.token).await {
        Err(e) => return ApiResponse::error("E_RESET_VERIFY", e.to_string(), 400),
        Ok(Some(e)) => e,
        Ok(None) => return ApiResponse::error("E_INVALID_TOKEN", "invalid token", 400),
    };

    let hash = match password::hash_async(body.password.clone()).await {
        Err(e) => return ApiResponse::error("E_HASH", e.to_string(), 500),
        Ok(h) => h,
    };

    match User::find_by_email(&email).await {
        Err(e) => ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(user)) => {
            match User::update_by_pk(user.id, &[("password_hash", rok_orm::SqlValue::Text(hash))]).await {
                Err(e) => ApiResponse::error("E_UPDATE", e.to_string(), 500),
                Ok(_) => ApiResponse::ok(serde_json::json!({ "message": "password reset" })),
            }
        }
    }
}
