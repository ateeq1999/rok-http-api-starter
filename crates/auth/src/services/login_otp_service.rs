use crate::context::AuthContext;
use crate::error::AuthError;
use crate::primitives;
use crate::primitives::TokenPair;

fn generate_otp(length: u32) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.gen_range(0..10).to_string())
        .collect()
}

pub async fn send_login_otp<C: AuthContext>(
    ctx: &C,
    email: &str,
) -> Result<(), AuthError> {
    let user = ctx.user_finder()
        .find_by_email(email)
        .await?
        .ok_or_else(|| AuthError::not_found("user not found"))?;

    let code = generate_otp(ctx.config().otp_length);
    let code_hash = primitives::sha256_hex(&code);
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

    let _ = sqlx::query("UPDATE login_otp SET used_at = NOW() WHERE email = $1 AND used_at IS NULL")
        .bind(&user.email)
        .execute(ctx.pool())
        .await;

    let _ = sqlx::query(
        "INSERT INTO login_otp (id, email, code_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(primitives::generate_id())
    .bind(&user.email)
    .bind(&code_hash)
    .bind(expires_at)
    .execute(ctx.pool())
    .await;

    let verify_url = format!(
        "{}/api/v1/auth/otp/login/verify?code={}&email={}",
        ctx.config().app_url, code, email,
    );

    if let Err(e) = ctx.mailer()
        .send_login_otp(&user.email, &user.name, &code, &verify_url)
        .await
    {
        tracing::error!("failed to send login OTP email to {}: {e}", user.email);
    } else {
        tracing::info!("login OTP email sent to {}", user.email);
    }

    Ok(())
}

pub async fn verify_login_otp<C: AuthContext>(
    ctx: &C,
    email: &str,
    code: &str,
) -> Result<TokenPair, AuthError> {
    let code_hash = primitives::sha256_hex(code);

    let record: (String, String) = sqlx::query_as(
        "SELECT id, email FROM login_otp
         WHERE email = $1 AND code_hash = $2 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(email)
    .bind(&code_hash)
    .fetch_optional(ctx.pool())
    .await?
    .ok_or_else(|| AuthError::bad_request("invalid or expired code"))?;

    let (otp_id, otp_email) = record;

    let _ = sqlx::query("UPDATE login_otp SET used_at = NOW() WHERE id = $1")
        .bind(&otp_id)
        .execute(ctx.pool())
        .await;

    let user = ctx.user_finder()
        .find_by_email(&otp_email)
        .await?
        .ok_or_else(|| AuthError::not_found("user not found"))?;

    let permissions = ctx.permission_finder().get_user_permissions(&user.id).await.unwrap_or_default();
    let family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &permissions,
        &ctx.config().auth_secret,
        ctx.config().token_ttl,
        ctx.config().refresh_ttl,
        Some(family_id),
    )
    .map_err(AuthError::internal)
}
