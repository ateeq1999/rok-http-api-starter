use crate::context::AuthContext;
use crate::error::AuthError;
use crate::primitives;
use totp_rs::{Algorithm, Secret, TOTP};

const TOTP_ISSUER: &str = "rok-api";
const TOTP_DIGITS: usize = 6;
const TOTP_SKEW: u8 = 1;
const TOTP_PERIOD: u64 = 30;
const BACKUP_CODE_COUNT: usize = 10;

fn generate_totp(secret_b32: &str) -> Result<TOTP, AuthError> {
    let secret = Secret::Encoded(secret_b32.to_string());
    let secret_bytes = secret.to_bytes().map_err(AuthError::internal)?;
    TOTP::new(
        Algorithm::SHA1,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_PERIOD,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        "user".to_string(),
    )
    .map_err(AuthError::internal)
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

pub async fn enable<C: AuthContext>(ctx: &C, user_id: &str) -> Result<EnableResult, AuthError> {
    let user = ctx.user_finder().find_or_fail(user_id).await?;

    if user.totp_enabled {
        return Err(AuthError::bad_request("2FA is already enabled"));
    }

    let totp_secret = Secret::generate_secret();
    let secret_b32 = totp_secret.to_encoded().to_string();
    let secret_bytes = totp_secret.to_bytes().map_err(AuthError::internal)?;

    let totp = TOTP::new(
        Algorithm::SHA1,
        TOTP_DIGITS,
        TOTP_SKEW,
        TOTP_PERIOD,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        user.email.clone(),
    )
    .map_err(AuthError::internal)?;
    let otpauth_url = totp.get_url();

    ctx.user_finder().update_user(user_id, &[("totp_secret", Some(&secret_b32))]).await?;

    let codes = generate_backup_codes();
    for code in &codes {
        let code_hash = primitives::sha256_hex(code);
        let _ = sqlx::query(
            "INSERT INTO two_factor_backup_codes (id, user_id, code_hash) VALUES ($1, $2, $3)",
        )
        .bind(primitives::generate_id())
        .bind(user_id)
        .bind(&code_hash)
        .execute(ctx.pool())
        .await;
    }

    Ok(EnableResult {
        secret: secret_b32,
        otpauth_url,
        backup_codes: codes,
    })
}

pub async fn verify_and_activate<C: AuthContext>(
    ctx: &C,
    user_id: &str,
    code: &str,
) -> Result<(), AuthError> {
    let user = ctx.user_finder().find_or_fail(user_id).await?;

    let secret = user.totp_secret
        .ok_or_else(|| AuthError::bad_request("2FA not initiated — call /auth/2fa/enable first"))?;

    let totp = generate_totp(&secret)?;
    if !totp.check_current(code).map_err(AuthError::internal)? {
        return Err(AuthError::bad_request("invalid TOTP code"));
    }

    ctx.user_finder().update_user(user_id, &[("totp_enabled", Some("true"))]).await?;

    Ok(())
}

pub async fn verify_code<C: AuthContext>(
    ctx: &C,
    user_id: &str,
    code: &str,
) -> Result<bool, AuthError> {
    let user = ctx.user_finder().find_or_fail(user_id).await?;

    if !user.totp_enabled {
        return Ok(true);
    }

    if let Some(ref secret) = user.totp_secret {
        let totp = generate_totp(secret)?;
        if totp.check_current(code).map_err(AuthError::internal)? {
            return Ok(true);
        }
    }

    let backup_hash = primitives::sha256_hex(code);
    let backup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM two_factor_backup_codes
         WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL
         LIMIT 1",
    )
    .bind(user_id)
    .bind(&backup_hash)
    .fetch_optional(ctx.pool())
    .await?;

    if let Some((id,)) = backup {
        let _ = sqlx::query("UPDATE two_factor_backup_codes SET used_at = NOW() WHERE id = $1")
            .bind(&id)
            .execute(ctx.pool())
            .await;
        return Ok(true);
    }

    Ok(false)
}

pub async fn disable<C: AuthContext>(
    ctx: &C,
    user_id: &str,
    password: &str,
    code: &str,
) -> Result<(), AuthError> {
    let user = ctx.user_finder().find_or_fail(user_id).await?;

    if !user.totp_enabled {
        return Err(AuthError::bad_request("2FA is not enabled"));
    }

    if !primitives::verify_password(password, &user.password_hash)
        .map_err(AuthError::internal)?
    {
        return Err(AuthError::unauthorized("invalid password"));
    }

    if let Some(ref secret) = user.totp_secret {
        let totp = generate_totp(secret)?;
        if !totp.check_current(code).map_err(AuthError::internal)? {
            return Err(AuthError::bad_request("invalid TOTP code"));
        }
    }

    ctx.user_finder().update_user(user_id, &[
        ("totp_secret", None),
        ("totp_enabled", Some("false")),
    ]).await?;

    let _ = sqlx::query("DELETE FROM two_factor_backup_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(ctx.pool())
        .await;

    Ok(())
}
