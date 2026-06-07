use axum::extract::State;
use axum::Json;

use crate::auth::{self, AuthUser};
use crate::db;
use crate::models::User;
use crate::response::{ApiResponse, ErrorCode};
use crate::services::crud::FieldValue;
use crate::services::crud::CrudService;
use crate::state::AppState;
use crate::validators;
use crate::validators::auth::*;

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<ApiResponse, validators::ValidationRejection> {
    let body = validators::validate(body)?;

    match User::find_by_email(&body.email).await {
        Ok(Some(_)) => return Ok(ApiResponse::error(ErrorCode::Conflict, "email already taken")),
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(None) => {}
    }

    let hash = match auth::hash_password(&body.password) {
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(h) => h,
    };

    let user = match User::create(&[
        ("id", FieldValue::String(auth::generate_id())),
        ("email", FieldValue::String(body.email.to_lowercase())),
        ("password_hash", FieldValue::String(hash)),
        ("name", FieldValue::String(body.name.clone())),
        ("roles", FieldValue::String("user".to_string())),
    ])
    .await
    {
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(u) => u,
    };

    match auth::generate_token_pair(
        &user.id,
        &user.roles,
        &state.config.auth_secret,
        state.config.token_ttl,
        state.config.refresh_ttl,
    ) {
        Err(e) => Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(tokens) => Ok(ApiResponse::ok(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
        }))),
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<ApiResponse, validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let user = match User::find_by_email(&body.email).await {
        Ok(Some(u)) => u,
        Ok(None) => return Ok(ApiResponse::error(ErrorCode::Unauthorized, "invalid email or password")),
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
    };

    match auth::verify_password(&body.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => return Ok(ApiResponse::error(ErrorCode::Unauthorized, "invalid email or password")),
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
    }

    match auth::generate_token_pair(
        &user.id,
        &user.roles,
        &state.config.auth_secret,
        state.config.token_ttl,
        state.config.refresh_ttl,
    ) {
        Err(e) => Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(tokens) => Ok(ApiResponse::ok(serde_json::json!({
            "access_token": tokens.access_token,
            "refresh_token": tokens.refresh_token,
        }))),
    }
}

pub async fn logout(_user: AuthUser) -> ApiResponse {
    ApiResponse::ok(serde_json::json!({ "message": "logged out" }))
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(body): Json<ForgotPasswordRequest>,
) -> Result<ApiResponse, validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let user = User::find_by_email(&body.email).await.unwrap_or(None);

    if let Some(user) = user {
        let plain_token = auth::generate_id();
        let token_hash = auth::sha256_hex(&plain_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        let pool = db::pool();
        let result = sqlx::query(
            "INSERT INTO password_resets (id, email, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(auth::generate_id())
        .bind(&user.email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(pool)
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

    Ok(ApiResponse::ok(serde_json::json!({ "message": "reset link sent" })))
}

pub async fn reset_password(
    Json(body): Json<ResetPasswordRequest>,
) -> Result<ApiResponse, validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let token_hash = auth::sha256_hex(&body.token);

    let pool = db::pool();
    let email: Option<String> = sqlx::query_scalar(
        "SELECT email FROM password_resets
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let email = match email {
        Some(e) => e,
        None => return Ok(ApiResponse::error(ErrorCode::BadRequest, "invalid or expired token")),
    };

    let hash = match auth::hash_password(&body.password) {
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(h) => h,
    };

    let user = match User::find_by_email(&email).await {
        Ok(Some(u)) => u,
        Ok(None) => return Ok(ApiResponse::error(ErrorCode::NotFound, "user not found")),
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
    };

    match User::update(&user.id, &[("password_hash", FieldValue::String(hash))]).await {
        Err(e) => Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(_) => {
            let _ = sqlx::query(
                "UPDATE password_resets SET used_at = NOW() WHERE token_hash = $1",
            )
            .bind(&token_hash)
            .execute(pool)
            .await;

            Ok(ApiResponse::ok(serde_json::json!({ "message": "password reset" })))
        }
    }
}
