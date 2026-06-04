use axum::extract::State;
use rok_auth::axum::GuestOnly;
use rok_auth::axum::RequestContext;
use rok_core::api::ApiResponse;
use rok_orm::Model;
use rok_orm::PgModel;
use rok_orm::SqlValue;
use rok_validate::Valid;

use crate::error::AppError;
use crate::models::EmailVerificationToken;
use crate::models::User;
use crate::state::AppState;
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
    ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<SendOtpRequest>,
) -> Result<ApiResponse, AppError> {
    let user = User::find_by_email(&body.email)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let code = generate_otp(state.config.otp_length);
    let hash = rok_auth::hash::sha256_hex(&code);
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    // Invalidate previous unused tokens via QueryBuilder DSL.
    EmailVerificationToken::update_where(
        ctx.db(),
        EmailVerificationToken::query()
            .where_eq("user_id", user.id.clone())
            .where_null("used_at"),
        &[("used_at", SqlValue::Text(chrono::Utc::now().to_rfc3339()))],
    )
    .await?;

    // Insert new token via pool-free Model DSL.
    EmailVerificationToken::create(&[
        (
            "id",
            SqlValue::Text(rok_core::crypto::Cuid2::generate().to_string()),
        ),
        ("user_id", SqlValue::Text(user.id.clone())),
        ("token_hash", SqlValue::Text(hash)),
        ("expires_at", SqlValue::Text(expires_at.to_rfc3339())),
    ])
    .await?;

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

    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "verification email sent" }),
    ))
}

pub async fn verify(
    _state: State<AppState>,
    _ctx: RequestContext,
    _: GuestOnly,
    Valid(body): Valid<VerifyOtpRequest>,
) -> Result<ApiResponse, AppError> {
    let user = User::find_by_email(&body.email)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let hash = rok_auth::hash::sha256_hex(&body.code);

    // Look up valid token via ModelQuery DSL (pool-free).
    let token = EmailVerificationToken::filter("user_id", SqlValue::Text(user.id.clone()))
        .and_where("token_hash", SqlValue::Text(hash))
        .and_where_null("used_at")
        .and_where_op(
            "expires_at",
            ">",
            SqlValue::Text(chrono::Utc::now().to_rfc3339()),
        )
        .first()
        .await?
        .ok_or_else(|| AppError::BadRequest("invalid or expired code".into()))?;

    // Mark token as used via pool-free Model DSL.
    EmailVerificationToken::update_by_pk(
        token.id,
        &[("used_at", SqlValue::Text(chrono::Utc::now().to_rfc3339()))],
    )
    .await?;

    // Mark user email as verified via pool-free Model DSL.
    User::update_by_pk(
        user.id,
        &[(
            "email_verified_at",
            SqlValue::Text(chrono::Utc::now().to_rfc3339()),
        )],
    )
    .await?;

    Ok(ApiResponse::ok(
        serde_json::json!({ "message": "email verified" }),
    ))
}
