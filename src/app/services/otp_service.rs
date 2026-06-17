use api_core::auth;
use api_core::crud::FieldValue;
use api_core::crud::CrudService;

use crate::app::models::EmailVerificationToken;
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

pub async fn send(
    config: &AppConfig,
    mailer: &Mailer,
    email: &str,
) -> Result<(), AppError> {
    let user = User::find_by_email(email)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let code = generate_otp(config.otp_length);
    let hash = auth::sha256_hex(&code);
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    EmailVerificationToken::invalidate_previous(&user.id)
        .await
        .or_internal()?;

    EmailVerificationToken::create(&[
        ("id", FieldValue::String(auth::generate_id())),
        ("user_id", FieldValue::String(user.id.clone())),
        ("token_hash", FieldValue::String(hash)),
        ("expires_at", FieldValue::DateTime(expires_at)),
    ])
    .await
    .or_internal()?;

    let verify_url = format!(
        "{}/api/v1/otp/verify?code={}&email={}",
        config.app_url, code, email
    );

    if let Err(e) = mailer
        .send_otp(email, &user.name, &code, &verify_url)
        .await
    {
        tracing::error!("failed to send OTP email: {e}");
    }

    Ok(())
}

pub async fn verify(email: &str, code: &str) -> Result<(), AppError> {
    let user = User::find_by_email(email)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let hash = auth::sha256_hex(code);

    let token = EmailVerificationToken::find_valid(&user.id, &hash)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::BadRequest("invalid or expired code".into()))?;

    EmailVerificationToken::mark_used(&token.id)
        .await
        .or_internal()?;

    User::verify_email(&user.id)
        .await
        .or_internal()?;

    Ok(())
}
