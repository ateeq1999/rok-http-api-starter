use auth::primitives;
use api_core::db;
use crate::app::models::User;
use crate::config::AppConfig;
use crate::app::mails::Mailer;
use crate::error::{AppError, OrInternal};

pub async fn request_magic_link(
    config: &AppConfig,
    mailer: &Mailer,
    email: &str,
) -> Result<(), AppError> {
    let user = User::find_by_email(email).await.or_internal()?;

    if let Some(user) = user {
        let plain_token = primitives::generate_id();
        let token_hash = primitives::sha256_hex(&plain_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::minutes(15);

        let _ = sqlx::query(
            "INSERT INTO magic_link_tokens (id, email, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(primitives::generate_id())
        .bind(&user.email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(db::pool())
        .await;

        let magic_url = format!(
            "{}/auth/magic-link/verify?token={}",
            config.app_url, plain_token,
        );

        if let Err(e) = mailer
            .send_magic_link(&user.email, &user.name, &magic_url)
            .await
        {
            tracing::error!("failed to send magic link email to {}: {e}", user.email);
        } else {
            tracing::info!("magic link email sent to {}", user.email);
        }
    }

    // Always return success to prevent email enumeration
    Ok(())
}

pub async fn verify_magic_link(
    config: &AppConfig,
    token: &str,
) -> Result<auth::primitives::TokenPair, AppError> {
    let token_hash = primitives::sha256_hex(token);

    let record: (String, String) = sqlx::query_as(
        "SELECT id, email FROM magic_link_tokens
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(db::pool())
    .await
    .or_internal()?
    .ok_or_else(|| AppError::BadRequest("invalid or expired magic link".into()))?;

    let (token_id, email) = record;

    // Mark token as used
    let _ = sqlx::query("UPDATE magic_link_tokens SET used_at = NOW() WHERE id = $1")
        .bind(&token_id)
        .execute(db::pool())
        .await;

    let user = User::find_by_email(&email)
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
