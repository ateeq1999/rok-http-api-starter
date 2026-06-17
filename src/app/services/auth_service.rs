use auth::primitives;
use api_core::crud::FieldValue;
use api_core::crud::CrudService;
use auth::primitives::TokenPair;

use api_core::db;
use crate::app::models::User;
use crate::config::AppConfig;
use crate::app::mails::Mailer;
use crate::error::{AppError, OrInternal};

fn to_internal<E: std::fmt::Display>(e: E) -> AppError {
    AppError::Internal(e.to_string())
}

pub async fn register(
    config: &AppConfig,
    email: &str,
    password: &str,
    name: &str,
) -> Result<TokenPair, AppError> {
    if User::find_by_email(email).await.or_internal()?.is_some() {
        return Err(AppError::BadRequest("email already taken".into()));
    }

    let hash = primitives::hash_password(password).map_err(to_internal)?;

    let user = User::create(&[
        ("id", FieldValue::String(primitives::generate_id())),
        ("email", FieldValue::String(email.to_lowercase())),
        ("password_hash", FieldValue::String(hash)),
        ("name", FieldValue::String(name.to_string())),
        ("roles", FieldValue::String("user".to_string())),
    ])
    .await
    .or_internal()?;

    let family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
        Some(family_id),
    )
    .map_err(to_internal)
}

pub async fn login(
    config: &AppConfig,
    identifier: &str,
    password: &str,
) -> Result<TokenPair, AppError> {
    let user = User::find_by_identifier(identifier)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::Unauthorized("invalid credentials".into()))?;

    if !primitives::verify_password(password, &user.password_hash).map_err(to_internal)? {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }

    let family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
        Some(family_id),
    )
    .map_err(to_internal)
}

pub async fn refresh(
    config: &AppConfig,
    refresh_token: &str,
) -> Result<TokenPair, AppError> {
    let claims = primitives::verify_token(refresh_token, &config.auth_secret)
        .map_err(|_| AppError::Unauthorized("invalid or expired refresh token".into()))?;

    let token_hash = primitives::sha256_hex(refresh_token);
    let family_id = claims.family_id.as_deref().unwrap_or("");

    // Check if family is revoked (replay attack detected upstream)
    if !family_id.is_empty() {
        let family_revoked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM refresh_token_revoked_families WHERE family_id = $1)",
        )
        .bind(family_id)
        .fetch_one(db::pool())
        .await
        .unwrap_or(false);

        if family_revoked {
            // Revoke all sessions for this user — token chain compromised
            let _ = sqlx::query(
                "UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(&claims.sub)
            .execute(db::pool())
            .await;
            return Err(AppError::Unauthorized("token family revoked — all sessions terminated".into()));
        }
    }

    // Check if this exact token was already used (replay)
    let already_used: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM refresh_token_used WHERE token_hash = $1)",
    )
    .bind(&token_hash)
    .fetch_one(db::pool())
    .await
    .unwrap_or(false);

    if already_used {
        // Token reuse detected — revoke entire family
        if !family_id.is_empty() {
            let _ = sqlx::query(
                "INSERT INTO refresh_token_revoked_families (id, family_id) VALUES ($1, $2)
                 ON CONFLICT (family_id) DO NOTHING",
            )
            .bind(primitives::generate_id())
            .bind(family_id)
            .execute(db::pool())
            .await;

            // Revoke all sessions for this user
            let _ = sqlx::query(
                "UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(&claims.sub)
            .execute(db::pool())
            .await;
        }
        return Err(AppError::Unauthorized("refresh token reuse detected — session terminated".into()));
    }

    // Mark current token as used, with its family_id
    let _ = sqlx::query(
        "INSERT INTO refresh_token_used (id, token_hash, family_id) VALUES ($1, $2, $3)",
    )
    .bind(primitives::generate_id())
    .bind(&token_hash)
    .bind(family_id)
    .execute(db::pool())
    .await;

    let user = User::find_or_fail(&claims.sub).await.or_internal()?;

    // Generate new token pair with a new family_id
    let new_family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
        Some(new_family_id),
    )
    .map_err(to_internal)
}

pub async fn forgot_password(
    config: &AppConfig,
    mailer: &Mailer,
    email: &str,
) -> Result<(), AppError> {
    let user = User::find_by_email(email).await.or_internal()?;

    if let Some(user) = user {
        let plain_token = primitives::generate_id();
        let token_hash = primitives::sha256_hex(&plain_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        match sqlx::query(
            "INSERT INTO password_resets (id, email, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(primitives::generate_id())
        .bind(&user.email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(db::pool())
        .await
        {
            Ok(_) => {
                let reset_url = format!(
                    "{}/reset-password?token={}",
                    config.app_url, plain_token,
                );
                if let Err(e) = mailer
                    .send_password_reset(&user.email, &user.name, &plain_token, &reset_url)
                    .await
                {
                    tracing::error!("failed to send password reset email to {}: {e}", user.email);
                } else {
                    tracing::info!("password reset email sent to {}", user.email);
                }
            }
            Err(e) => {
                tracing::error!("failed to insert password reset token: {e}");
            }
        }
    }

    Ok(())
}

pub async fn reset_password(
    token: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let token_hash = primitives::sha256_hex(token);

    let email: String = sqlx::query_scalar(
        "SELECT email FROM password_resets
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(db::pool())
    .await
    .or_internal()?
    .ok_or_else(|| AppError::BadRequest("invalid or expired token".into()))?;

    let hash = primitives::hash_password(new_password).map_err(to_internal)?;

    let user = User::find_by_email(&email)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    User::update(&user.id, &[("password_hash", FieldValue::String(hash))])
        .await
        .or_internal()?;

    let _ = sqlx::query(
        "UPDATE password_resets SET used_at = NOW() WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .execute(db::pool())
    .await;

    Ok(())
}
