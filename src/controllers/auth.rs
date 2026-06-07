use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::auth::{self, AuthUser};
use crate::models::User;
use crate::response;
use crate::services::crud::{CrudService, FieldValue};
use crate::state::AppState;
use crate::validators;
use crate::validators::auth::*;

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    match User::find_by_email(&state.pool, &body.email).await {
        Ok(Some(_)) => return Ok(response::error("E_DUPLICATE_EMAIL", "email already taken", 409)),
        Err(e) => return Ok(response::error("E_DATABASE", &e.to_string(), 500)),
        Ok(None) => {}
    }

    let hash = match auth::hash_password(&body.password) {
        Err(e) => return Ok(response::error("E_REGISTRATION", &e.to_string(), 500)),
        Ok(h) => h,
    };

    let user = match User::create(
        &state.pool,
        &[
            ("id", FieldValue::String(auth::generate_id())),
            ("email", FieldValue::String(body.email.to_lowercase())),
            ("password_hash", FieldValue::String(hash)),
            ("name", FieldValue::String(body.name.clone())),
            ("roles", FieldValue::String("user".to_string())),
        ],
    )
    .await
    {
        Err(e) => return Ok(response::error("E_DATABASE", &e.to_string(), 500)),
        Ok(u) => u,
    };

    match auth::generate_token_pair(
        &user.id,
        &user.roles,
        &state.config.auth_secret,
        state.config.token_ttl,
        state.config.refresh_ttl,
    ) {
        Err(e) => Ok(response::error("E_LOGIN", &e.to_string(), 500)),
        Ok(tokens) => Ok(response::ok(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
        }))),
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let user = match User::find_by_email(&state.pool, &body.email).await {
        Ok(Some(u)) => u,
        Ok(None) => return Ok(response::error("E_LOGIN", "invalid email or password", 401)),
        Err(e) => return Ok(response::error("E_DATABASE", &e.to_string(), 500)),
    };

    match auth::verify_password(&body.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => return Ok(response::error("E_LOGIN", "invalid email or password", 401)),
        Err(e) => return Ok(response::error("E_LOGIN", &e.to_string(), 500)),
    }

    match auth::generate_token_pair(
        &user.id,
        &user.roles,
        &state.config.auth_secret,
        state.config.token_ttl,
        state.config.refresh_ttl,
    ) {
        Err(e) => Ok(response::error("E_LOGIN", &e.to_string(), 500)),
        Ok(tokens) => Ok(response::ok(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
        }))),
    }
}

pub async fn logout(_user: AuthUser) -> (axum::http::StatusCode, Json<Value>) {
    response::ok(serde_json::json!({ "message": "logged out" }))
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let user = User::find_by_email(&state.pool, &body.email)
        .await
        .unwrap_or(None);

    if let Some(user) = user {
        let plain_token = auth::generate_id();
        let token_hash = auth::sha256_hex(&plain_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        let result = sqlx::query(
            "INSERT INTO password_resets (id, email, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(auth::generate_id())
        .bind(&user.email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&state.pool)
        .await;

        if let Ok(_) = result {
            let reset_url = format!(
                "{}/reset-password?token={}",
                state.config.app_url, plain_token,
            );
            if let Err(e) = state
                .mailer
                .send_password_reset(&user.email, &user.name, &plain_token, &reset_url)
                .await
            {
                tracing::error!("failed to send password reset email: {e}");
            }
        }
    }

    Ok(response::ok(serde_json::json!({ "message": "reset link sent" })))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(body): Json<ResetPasswordRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let token_hash = auth::sha256_hex(&body.token);

    let email: Option<String> = sqlx::query_scalar(
        "SELECT email FROM password_resets
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let email = match email {
        Some(e) => e,
        None => return Ok(response::error("E_INVALID_TOKEN", "invalid or expired token", 400)),
    };

    let hash = match auth::hash_password(&body.password) {
        Err(e) => return Ok(response::error("E_HASH", &e.to_string(), 500)),
        Ok(h) => h,
    };

    let user = match User::find_by_email(&state.pool, &email).await {
        Ok(Some(u)) => u,
        Ok(None) => return Ok(response::error("E_ROW_NOT_FOUND", "user not found", 404)),
        Err(e) => return Ok(response::error("E_DATABASE", &e.to_string(), 500)),
    };

    match User::update(
        &state.pool,
        &user.id,
        &[("password_hash", FieldValue::String(hash))],
    )
    .await
    {
        Err(e) => Ok(response::error("E_UPDATE", &e.to_string(), 500)),
        Ok(_) => {
            let _ = sqlx::query(
                "UPDATE password_resets SET used_at = NOW() WHERE token_hash = $1",
            )
            .bind(&token_hash)
            .execute(&state.pool)
            .await;

            Ok(response::ok(serde_json::json!({ "message": "password reset" })))
        }
    }
}
