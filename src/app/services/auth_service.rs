use api_core::auth;
use api_core::crud::FieldValue;
use api_core::crud::CrudService;

use api_core::db;
use crate::app::models::User;
use crate::config::AppConfig;
use crate::app::mails::Mailer;
use crate::error::AppError;

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn register(
    config: &AppConfig,
    email: &str,
    password: &str,
    name: &str,
) -> Result<TokenPair, AppError> {
    match User::find_by_email(email).await {
        Ok(Some(_)) => return Err(AppError::BadRequest("email already taken".into())),
        Err(e) => return Err(AppError::Database(e.to_string())),
        Ok(None) => {}
    }

    let hash = auth::hash_password(password)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = User::create(&[
        ("id", FieldValue::String(auth::generate_id())),
        ("email", FieldValue::String(email.to_lowercase())),
        ("password_hash", FieldValue::String(hash)),
        ("name", FieldValue::String(name.to_string())),
        ("roles", FieldValue::String("user".to_string())),
    ])
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    let tokens = auth::generate_token_pair(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(TokenPair {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
}

pub async fn login(
    config: &AppConfig,
    email: &str,
    password: &str,
) -> Result<TokenPair, AppError> {
    let user = User::find_by_email(email)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Unauthorized("invalid email or password".into()))?;

    let valid = auth::verify_password(password, &user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !valid {
        return Err(AppError::Unauthorized("invalid email or password".into()));
    }

    let tokens = auth::generate_token_pair(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(TokenPair {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
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

    let user = User::find_by_id(&claims.sub)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Unauthorized("user not found".into()))?;

    let tokens = auth::generate_token_pair(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(TokenPair {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
}

pub async fn forgot_password(
    config: &AppConfig,
    mailer: &Mailer,
    email: &str,
) -> Result<(), AppError> {
    let user = User::find_by_email(email).await.unwrap_or(None);

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
        .execute(db::pool())
        .await;

        if let Ok(_) = result {
            let reset_url = format!(
                "{}/reset-password?token={}",
                config.app_url, plain_token,
            );
            if let Err(e) = mailer
                .send_password_reset(&user.email, &user.name, &plain_token, &reset_url)
                .await
            {
                tracing::error!("failed to send password reset email: {e}");
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
    .map_err(|e| AppError::Database(e.to_string()))?
    .ok_or_else(|| AppError::BadRequest("invalid or expired token".into()))?;

    let hash = auth::hash_password(new_password)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = User::find_by_email(&email)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    User::update(&user.id, &[("password_hash", FieldValue::String(hash))])
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let _ = sqlx::query(
        "UPDATE password_resets SET used_at = NOW() WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .execute(db::pool())
    .await;

    Ok(())
}
