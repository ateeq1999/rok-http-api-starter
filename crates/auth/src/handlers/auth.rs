use api_core::response::ApiResponse;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::context::AuthContext;
use crate::error::AuthError;
use crate::extractors::AuthUser;
use super::token_response;
use crate::services;
use crate::validators::ValidatedJson;
use crate::validators::{
    ForgotPasswordRequest, LoginRequest, RefreshRequest, RegisterRequest, ResetPasswordRequest,
};

pub async fn register<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::auth_service::register(&ctx, &body.email, &body.password, &body.name).await?;
    Ok(token_response(&ctx, &tokens, "registered"))
}

pub async fn login<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::auth_service::login(&ctx, &body.email, &body.password).await?;
    Ok(token_response(&ctx, &tokens, "logged in"))
}

pub async fn refresh<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<RefreshRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::auth_service::refresh(&ctx, &body.refresh_token).await?;
    Ok(token_response(&ctx, &tokens, "refreshed"))
}

pub async fn logout<C: AuthContext>(
    State(ctx): State<C>,
    _user: AuthUser,
) -> Response {
    use crate::middleware::AuthStrategy;
    use axum::http::header;

    if ctx.config().auth_strategy == AuthStrategy::Cookie {
        let body = ApiResponse::ok(serde_json::json!({ "message": "logged out" }));
        let mut response = body.into_response();
        let headers = response.headers_mut();
        let clear_access = "access_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
        let clear_refresh = "refresh_token=; Path=/api/v1/auth/refresh; HttpOnly; SameSite=Lax; Max-Age=0";
        headers.append(header::SET_COOKIE, clear_access.parse().unwrap());
        headers.append(header::SET_COOKIE, clear_refresh.parse().unwrap());
        response
    } else {
        ApiResponse::ok(serde_json::json!({ "message": "logged out" })).into_response()
    }
}

pub async fn forgot_password<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<ForgotPasswordRequest>,
) -> Result<ApiResponse, AuthError> {
    services::auth_service::forgot_password(&ctx, &body.email).await?;
    Ok(ApiResponse::message("reset link sent"))
}

pub async fn reset_password<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<ResetPasswordRequest>,
) -> Result<ApiResponse, AuthError> {
    services::auth_service::reset_password(&ctx, &body.token, &body.password).await?;
    Ok(ApiResponse::message("password reset"))
}
