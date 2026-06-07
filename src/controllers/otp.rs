use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::auth;
use crate::error::AppError;
use crate::models::EmailVerificationToken;
use crate::models::User;
use crate::response;
use crate::state::AppState;
use crate::validators;
use crate::validators::otp::*;

fn generate_otp(length: u32) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.gen_range(0..10).to_string())
        .collect()
}

pub async fn send(
    State(state): State<AppState>,
    Json(body): Json<SendOtpRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let body = validators::validate(body).map_err(|_| AppError::BadRequest("invalid request".into()))?;

    let user = User::find_by_email(&state.pool, &body.email)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let code = generate_otp(state.config.otp_length);
    let hash = auth::sha256_hex(&code);
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    EmailVerificationToken::invalidate_previous(&state.pool, &user.id).await?;

    EmailVerificationToken::create(&state.pool, &user.id, &hash, &expires_at).await?;

    let verify_url = format!(
        "{}/api/v1/otp/verify?code={}&email={}",
        state.config.app_url, code, body.email
    );

    if let Err(e) = state
        .mailer
        .send_otp(&body.email, &user.name, &code, &verify_url)
        .await
    {
        tracing::error!("failed to send OTP email: {e}");
    }

    Ok(response::ok(
        serde_json::json!({ "message": "verification email sent" }),
    ))
}

pub async fn verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyOtpRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let body = validators::validate(body).map_err(|_| AppError::BadRequest("invalid request".into()))?;

    let user = User::find_by_email(&state.pool, &body.email)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let hash = auth::sha256_hex(&body.code);

    let token = EmailVerificationToken::find_valid(&state.pool, &user.id, &hash)
        .await?
        .ok_or_else(|| AppError::BadRequest("invalid or expired code".into()))?;

    EmailVerificationToken::mark_used(&state.pool, &token.id).await?;

    User::verify_email(&state.pool, &user.id).await?;

    Ok(response::ok(
        serde_json::json!({ "message": "email verified" }),
    ))
}
