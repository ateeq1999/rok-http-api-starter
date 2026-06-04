use axum::extract::State;
use rok_auth::axum::GuestOnly;
use rok_auth::axum::RequestContext;
use rok_core::api::ApiResponse;
use rok_validate::Valid;

use crate::models::User;
use crate::state::AppState;
use crate::validators::otp::*;

fn generate_otp(length: u32) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length).map(|_| rng.gen_range(0..10).to_string()).collect()
}

pub async fn send(
    State(state): State<AppState>,
    ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<SendOtpRequest>,
) -> ApiResponse {
    let user = match User::find_by_email(&body.email).await {
        Err(e) => return ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => return ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(u)) => u,
    };

    let code = generate_otp(state.config.otp_length);
    let hash = rok_auth::hash::sha256_hex(&code);
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    // Invalidate previous unused tokens.
    if let Err(e) = sqlx::query(
        "UPDATE email_verification_tokens SET used_at = now() WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(&user.id)
    .execute(ctx.db())
    .await
    {
        return ApiResponse::error("E_DATABASE", e.to_string(), 500);
    }

    if let Err(e) = sqlx::query(
        "INSERT INTO email_verification_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(&user.id)
    .bind(&hash)
    .bind(expires_at)
    .execute(ctx.db())
    .await
    {
        return ApiResponse::error("E_DATABASE", e.to_string(), 500);
    }

    let verify_url = format!(
        "{}/verify-email?code={}&email={}",
        state.config.app_url, code, body.email
    );

    if let Err(e) = state
        .mailer
        .send_otp(&body.email, &user.name, &code, &verify_url)
        .await
    {
        tracing::error!("failed to send OTP email: {e}");
    }

    ApiResponse::ok(serde_json::json!({ "message": "verification email sent" }))
}

pub async fn verify(
    _state: State<AppState>,
    ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<VerifyOtpRequest>,
) -> ApiResponse {
    let user = match User::find_by_email(&body.email).await {
        Err(e) => return ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => return ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(u)) => u,
    };

    let hash = rok_auth::hash::sha256_hex(&body.code);

    let row = match sqlx::query_as::<_, (String,)>(
        "SELECT id FROM email_verification_tokens
         WHERE user_id = $1 AND token_hash = $2 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(&user.id)
    .bind(&hash)
    .fetch_optional(ctx.db())
    .await
    {
        Err(e) => return ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(r) => r,
    };

    let Some((token_id,)) = row else {
        return ApiResponse::error("E_INVALID_OTP", "invalid or expired code", 400);
    };

    if let Err(e) = sqlx::query("UPDATE email_verification_tokens SET used_at = now() WHERE id = $1")
        .bind(&token_id)
        .execute(ctx.db())
        .await
    {
        return ApiResponse::error("E_DATABASE", e.to_string(), 500);
    }

    if let Err(e) = sqlx::query("UPDATE users SET email_verified_at = now() WHERE id = $1")
        .bind(&user.id)
        .execute(ctx.db())
        .await
    {
        return ApiResponse::error("E_DATABASE", e.to_string(), 500);
    }

    ApiResponse::ok(serde_json::json!({ "message": "email verified" }))
}
