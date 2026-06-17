use api_core::auth;
use api_core::crud::FieldValue;
use api_core::crud::CrudService;
use api_core::auth::TokenPair;

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

    let hash = auth::hash_password(password).map_err(to_internal)?;

    let user = User::create(&[
        ("id", FieldValue::String(auth::generate_id())),
        ("email", FieldValue::String(email.to_lowercase())),
        ("password_hash", FieldValue::String(hash)),
        ("name", FieldValue::String(name.to_string())),
        ("roles", FieldValue::String("user".to_string())),
    ])
    .await
    .or_internal()?;

    auth::generate_token_pair(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
    )
    .map_err(to_internal)
}

pub async fn login(
    config: &AppConfig,
    email: &str,
    password: &str,
) -> Result<TokenPair, AppError> {
    let user = User::find_by_email(email)
        .await
        .or_internal()?
        .ok_or_else(|| AppError::Unauthorized("invalid email or password".into()))?;

    if !auth::verify_password(password, &user.password_hash).map_err(to_internal)? {
        return Err(AppError::Unauthorized("invalid email or password".into()));
    }

    auth::generate_token_pair(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
    )
    .map_err(to_internal)
}

pub async fn refresh(
    config: &AppConfig,
    refresh_token: &str,
) -> Result<TokenPair, AppError> {
    let claims = auth::verify_token(refresh_token, &config.auth_secret)
        .map_err(|_| AppError::Unauthorized("invalid or expired refresh token".into()))?;

    let token_hash = auth::sha256_hex(refresh_token);

    let already_used: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM refresh_token_used WHERE token_hash = $1)",
    )
    .bind(&token_hash)
    .fetch_one(db::pool())
    .await
    .unwrap_or(false);

    if already_used {
        return Err(AppError::Unauthorized("refresh token already used".into()));
    }

    let _ = sqlx::query(
        "INSERT INTO refresh_token_used (id, token_hash) VALUES ($1, $2)",
    )
    .bind(auth::generate_id())
    .bind(&token_hash)
    .execute(db::pool())
    .await;

    let user = User::find_or_fail(&claims.sub).await.or_internal()?;

    auth::generate_token_pair(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
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
        let plain_token = auth::generate_id();
        let token_hash = auth::sha256_hex(&plain_token);
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

        match sqlx::query(
            "INSERT INTO password_resets (id, email, token_hash, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(auth::generate_id())
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
    let token_hash = auth::sha256_hex(token);

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

    let hash = auth::hash_password(new_password).map_err(to_internal)?;

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
