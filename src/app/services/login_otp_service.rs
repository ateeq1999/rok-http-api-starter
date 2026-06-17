use auth::primitives;
use api_core::db;
use crate::app::models::User;
use crate::config::AppConfig;
use crate::app::mails::Mailer;
use crate::error::{AppError, OrInternal};

fn generate_otp(length: u32) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.gen_range(0..10).to_string())
        .collect()
}

pub async fn send_login_otp(
    config: &AppConfig,
    mailer: &Mailer,
    email: &str,
) -> Result<(), AppError> {
    let user = User::find_by_email(email)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let code = generate_otp(config.otp_length);
    let code_hash = primitives::sha256_hex(&code);
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

    // Invalidate any previous unused OTPs for this email
    let _ = sqlx::query("UPDATE login_otp SET used_at = NOW() WHERE email = $1 AND used_at IS NULL")
        .bind(&user.email)
        .execute(db::pool())
        .await;

    let _ = sqlx::query(
        "INSERT INTO login_otp (id, email, code_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(primitives::generate_id())
    .bind(&user.email)
    .bind(&code_hash)
    .bind(expires_at)
    .execute(db::pool())
    .await;

    let verify_url = format!(
        "{}/api/v1/auth/otp/login/verify?code={}&email={}",
        config.app_url, code, email,
    );

    if let Err(e) = mailer
        .send_login_otp(&user.email, &user.name, &code, &verify_url)
        .await
    {
        tracing::error!("failed to send login OTP email to {}: {e}", user.email);
    } else {
        tracing::info!("login OTP email sent to {}", user.email);
    }

    Ok(())
}

pub async fn verify_login_otp(
    config: &AppConfig,
    email: &str,
    code: &str,
) -> Result<auth::primitives::TokenPair, AppError> {
    let code_hash = primitives::sha256_hex(code);

    let record: (String, String) = sqlx::query_as(
        "SELECT id, email FROM login_otp
         WHERE email = $1 AND code_hash = $2 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(email)
    .bind(&code_hash)
    .fetch_optional(db::pool())
    .await
    .or_internal()?
    .ok_or_else(|| AppError::BadRequest("invalid or expired code".into()))?;

    let (otp_id, otp_email) = record;

    // Mark OTP as used
    let _ = sqlx::query("UPDATE login_otp SET used_at = NOW() WHERE id = $1")
        .bind(&otp_id)
        .execute(db::pool())
        .await;

    let user = User::find_by_email(&otp_email)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
        Some(family_id),
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}
