use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use api_core::response::ApiResponse;
use auth::primitives::TokenPair;

use auth::extractors::AuthUser;
use crate::app::services;
use crate::config::AuthStrategy;
use crate::error::AppError;
use crate::state::AppState;
use auth::validators::ValidatedJson;
use auth::validators::{RefreshRequest, RegisterRequest, LoginRequest, ForgotPasswordRequest, ResetPasswordRequest};

fn token_response(state: &AppState, tokens: &TokenPair, message: &str) -> Response {
    if state.config.auth_strategy == AuthStrategy::Cookie {
        let access_cookie = format!(
            "access_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
            tokens.access_token,
            state.config.token_ttl.as_secs(),
        );
        let refresh_cookie = format!(
            "refresh_token={}; Path=/api/v1/auth/refresh; HttpOnly; SameSite=Lax; Max-Age={}",
            tokens.refresh_token,
            state.config.refresh_ttl.as_secs(),
        );
        let body = ApiResponse::ok(serde_json::json!({ "message": message }));
        let mut response = body.into_response();
        let headers = response.headers_mut();
        headers.append(header::SET_COOKIE, access_cookie.parse().unwrap());
        headers.append(header::SET_COOKIE, refresh_cookie.parse().unwrap());
        response
    } else {
        ApiResponse::ok(serde_json::json!(tokens)).into_response()
    }
}

pub async fn refresh(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RefreshRequest>,
) -> Result<Response, AppError> {
    let tokens = services::auth_service::refresh(&state.config, &body.refresh_token).await?;
    Ok(token_response(&state, &tokens, "refreshed"))
}

pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> Result<Response, AppError> {
    let tokens = services::auth_service::register(
        &state.config,
        &body.email,
        &body.password,
        &body.name,
    )
    .await?;
    Ok(token_response(&state, &tokens, "registered"))
}

pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> Result<Response, AppError> {
    let tokens = services::auth_service::login(&state.config, &body.email, &body.password).await?;
    Ok(token_response(&state, &tokens, "logged in"))
}

pub async fn logout(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Response {
    if state.config.auth_strategy == AuthStrategy::Cookie {
        let body = ApiResponse::ok(serde_json::json!({ "message": "logged out" }));
        let mut response = body.into_response();
        let headers = response.headers_mut();
        // Clear cookies by setting expired
        let clear_access = "access_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
        let clear_refresh = "refresh_token=; Path=/api/v1/auth/refresh; HttpOnly; SameSite=Lax; Max-Age=0";
        headers.append(header::SET_COOKIE, clear_access.parse().unwrap());
        headers.append(header::SET_COOKIE, clear_refresh.parse().unwrap());
        response
    } else {
        ApiResponse::ok(serde_json::json!({ "message": "logged out" })).into_response()
    }
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
