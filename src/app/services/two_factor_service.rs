use auth::primitives;
use api_core::db;
use api_core::crud::{CrudService, FieldValue};
use crate::app::models::User;
use crate::error::{AppError, OrInternal};
use totp_rs::{Algorithm, Secret, TOTP};

const TOTP_ISSUER: &str = "rok-api";
const TOTP_DIGITS: usize = 6;
const TOTP_SKEW: u8 = 1;
const TOTP_PERIOD: u64 = 30;
const BACKUP_CODE_COUNT: usize = 10;

fn generate_totp(secret_b32: &str) -> Result<TOTP, AppError> {
    let secret = Secret::Encoded(secret_b32.to_string());
    let secret_bytes = secret.to_bytes().map_err(|e| AppError::Internal(e.to_string()))?;
    TOTP::new(
        Algorithm::SHA1,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_PERIOD,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        "user".to_string(),
    )
    .map_err(|e| AppError::Internal(e.to_string()))
}

fn generate_backup_codes() -> Vec<String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..BACKUP_CODE_COUNT)
        .map(|_| format!("{:08}", rng.gen::<u32>()))
        .collect()
}

pub struct EnableResult {
    pub secret: String,
    pub otpauth_url: String,
    pub backup_codes: Vec<String>,
}

pub async fn enable(user_id: &str) -> Result<EnableResult, AppError> {
    let user = User::find_or_fail(user_id).await.or_internal()?;

    if user.totp_enabled {
        return Err(AppError::BadRequest("2FA is already enabled".into()));
    }

    // Generate new TOTP secret (base32 encoded)
    let totp_secret = Secret::generate_secret();
    let secret_b32 = totp_secret.to_encoded().to_string();
    let secret_bytes = totp_secret.to_bytes().map_err(|e| AppError::Internal(e.to_string()))?;

    // Generate otpauth URL
    let totp = TOTP::new(
        Algorithm::SHA1,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_PERIOD,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        user.email.clone(),
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;
    let otpauth_url = totp.get_url();

    // Store secret (not yet enabled — needs verification)
    User::update(user_id, &[
        ("totp_secret", FieldValue::String(secret_b32.clone())),
    ])
    .await
    .or_internal()?;

    // Generate and store backup codes
    let codes = generate_backup_codes();
    for code in &codes {
        let code_hash = primitives::sha256_hex(code);
        let _ = sqlx::query(
            "INSERT INTO two_factor_backup_codes (id, user_id, code_hash) VALUES ($1, $2, $3)",
        )
        .bind(primitives::generate_id())
        .bind(user_id)
        .bind(&code_hash)
        .execute(db::pool())
        .await;
    }

    Ok(EnableResult {
        secret: secret_b32,
        otpauth_url,
        backup_codes: codes,
    })
}

pub async fn verify_and_activate(user_id: &str, code: &str) -> Result<(), AppError> {
    let user = User::find_or_fail(user_id).await.or_internal()?;

    let secret = user.totp_secret
        .ok_or_else(|| AppError::BadRequest("2FA not initiated — call /auth/2fa/enable first".into()))?;

    let totp = generate_totp(&secret)?;
    if !totp.check_current(code).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::BadRequest("invalid TOTP code".into()));
    }

    User::update(user_id, &[
        ("totp_enabled", FieldValue::Bool(true)),
    ])
    .await
    .or_internal()?;

    Ok(())
}

pub async fn verify_code(user_id: &str, code: &str) -> Result<bool, AppError> {
    let user = User::find_or_fail(user_id).await.or_internal()?;

    if !user.totp_enabled {
        return Ok(true); // 2FA not enabled — skip check
    }

    // Try TOTP code first
    if let Some(ref secret) = user.totp_secret {
        let totp = generate_totp(secret)?;
        if totp.check_current(code).map_err(|e| AppError::Internal(e.to_string()))? {
            return Ok(true);
        }
    }

    // Try backup code
    let backup_hash = primitives::sha256_hex(code);
    let backup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM two_factor_backup_codes
         WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL
         LIMIT 1",
    )
    .bind(user_id)
    .bind(&backup_hash)
    .fetch_optional(db::pool())
    .await
    .or_internal()?;

    if let Some((id,)) = backup {
        let _ = sqlx::query("UPDATE two_factor_backup_codes SET used_at = NOW() WHERE id = $1")
            .bind(&id)
            .execute(db::pool())
            .await;
        return Ok(true);
    }

    Ok(false)
}

pub async fn disable(user_id: &str, password: &str, code: &str) -> Result<(), AppError> {
    let user = User::find_or_fail(user_id).await.or_internal()?;

    if !user.totp_enabled {
        return Err(AppError::BadRequest("2FA is not enabled".into()));
    }

    // Verify password
    if !primitives::verify_password(password, &user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?
    {
        return Err(AppError::Unauthorized("invalid password".into()));
    }

    // Verify TOTP code
    if let Some(ref secret) = user.totp_secret {
        let totp = generate_totp(secret)?;
        if !totp.check_current(code).map_err(|e| AppError::Internal(e.to_string()))? {
            return Err(AppError::BadRequest("invalid TOTP code".into()));
        }
    }

    // Disable 2FA
    User::update(user_id, &[
        ("totp_secret", FieldValue::OptionString(None)),
        ("totp_enabled", FieldValue::Bool(false)),
    ])
    .await
    .or_internal()?;

    // Delete backup codes
    let _ = sqlx::query("DELETE FROM two_factor_backup_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(db::pool())
        .await;

    Ok(())
}
