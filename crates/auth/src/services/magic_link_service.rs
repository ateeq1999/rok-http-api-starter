use crate::context::AuthContext;
use crate::error::AuthError;
use crate::primitives;
use crate::primitives::TokenPair;

pub async fn request_magic_link<C: AuthContext>(
    ctx: &C,
    email: &str,
) -> Result<(), AuthError> {
    let user = ctx.user_finder().find_by_email(email).await?;

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
        .execute(ctx.pool())
        .await;

        let magic_url = format!(
            "{}/auth/magic-link/verify?token={}",
            ctx.config().app_url, plain_token,
        );

        if let Err(e) = ctx.mailer()
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

pub async fn verify_magic_link<C: AuthContext>(
    ctx: &C,
    token: &str,
) -> Result<TokenPair, AuthError> {
    let token_hash = primitives::sha256_hex(token);

    let record: (String, String) = sqlx::query_as(
        "SELECT id, email FROM magic_link_tokens
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(ctx.pool())
    .await?
    .ok_or_else(|| AuthError::bad_request("invalid or expired magic link"))?;

    let (token_id, email) = record;

    let _ = sqlx::query("UPDATE magic_link_tokens SET used_at = NOW() WHERE id = $1")
        .bind(&token_id)
        .execute(ctx.pool())
        .await;

    let user = ctx.user_finder()
        .find_by_email(&email)
        .await?
        .ok_or_else(|| AuthError::not_found("user not found"))?;

    let family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &ctx.config().auth_secret,
        ctx.config().token_ttl,
        ctx.config().refresh_ttl,
        Some(family_id),
    )
    .map_err(AuthError::internal)
}
