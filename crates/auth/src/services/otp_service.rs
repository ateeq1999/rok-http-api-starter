use crate::context::AuthContext;
use crate::error::AuthError;
use crate::primitives;

fn generate_otp(length: u32) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| rng.gen_range(0..10).to_string())
        .collect()
}

pub async fn send<C: AuthContext>(
    ctx: &C,
    email: &str,
) -> Result<(), AuthError> {
    let code = generate_otp(ctx.config().otp_length);
    let code_hash = primitives::sha256_hex(&code);
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

    let _ = sqlx::query("UPDATE otp_verifications SET used_at = NOW() WHERE email = $1 AND used_at IS NULL")
        .bind(email)
        .execute(ctx.pool())
        .await;

    let _ = sqlx::query(
        "INSERT INTO otp_verifications (id, email, code_hash, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(primitives::generate_id())
    .bind(email)
    .bind(&code_hash)
    .bind(expires_at)
    .execute(ctx.pool())
    .await;

    let verify_url = format!(
        "{}/api/v1/otp/verify?code={}&email={}",
        ctx.config().app_url, code, email,
    );

    if let Err(e) = ctx.mailer()
        .send_otp(email, "", &code, &verify_url)
        .await
    {
        tracing::error!("failed to send OTP email to {email}: {e}");
    } else {
        tracing::info!("OTP email sent to {email}");
    }

    Ok(())
}

pub async fn verify<C: AuthContext>(
    ctx: &C,
    email: &str,
    code: &str,
) -> Result<(), AuthError> {
    let code_hash = primitives::sha256_hex(code);

    let record: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM otp_verifications
         WHERE email = $1 AND code_hash = $2 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(email)
    .bind(&code_hash)
    .fetch_optional(ctx.pool())
    .await?;

    let (otp_id,) = record.ok_or_else(|| AuthError::bad_request("invalid or expired code"))?;

    let _ = sqlx::query("UPDATE otp_verifications SET used_at = NOW() WHERE id = $1")
        .bind(&otp_id)
        .execute(ctx.pool())
        .await;

    let _ = sqlx::query("UPDATE users SET email_verified_at = NOW(), updated_at = NOW() WHERE email = $1")
        .bind(email)
        .execute(ctx.pool())
        .await;

    Ok(())
}
