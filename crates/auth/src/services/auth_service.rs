use crate::context::AuthContext;
use crate::error::AuthError;
use crate::primitives;
use crate::primitives::TokenPair;

pub async fn register<C: AuthContext>(
    ctx: &C,
    email: &str,
    password: &str,
    name: &str,
) -> Result<TokenPair, AuthError> {
    let finder = ctx.user_finder();
    if finder.find_by_email(email).await?.is_some() {
        return Err(AuthError::bad_request("email already taken"));
    }

    let hash = primitives::hash_password(password).map_err(AuthError::internal)?;

    let user = finder.create_user(&[
        ("id", &primitives::generate_id()),
        ("email", &email.to_lowercase()),
        ("password_hash", &hash),
        ("name", name),
        ("roles", "user"),
    ]).await?;

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

pub async fn login<C: AuthContext>(
    ctx: &C,
    identifier: &str,
    password: &str,
) -> Result<TokenPair, AuthError> {
    let user = ctx.user_finder()
        .find_by_identifier(identifier)
        .await?
        .ok_or_else(|| AuthError::unauthorized("invalid credentials"))?;

    if !primitives::verify_password(password, &user.password_hash)
        .map_err(AuthError::internal)?
    {
        return Err(AuthError::unauthorized("invalid credentials"));
    }

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

pub async fn refresh<C: AuthContext>(
    ctx: &C,
    refresh_token: &str,
) -> Result<TokenPair, AuthError> {
    let config = ctx.config();
    let pool = ctx.pool();

    let claims = primitives::verify_token(refresh_token, &config.auth_secret)
        .map_err(|_| AuthError::unauthorized("invalid or expired refresh token"))?;

    let token_hash = primitives::sha256_hex(refresh_token);
    let family_id = claims.family_id.as_deref().unwrap_or("");

    // Check if family is revoked (replay attack detected upstream)
    if !family_id.is_empty() {
        let family_revoked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM refresh_token_revoked_families WHERE family_id = $1)",
        )
        .bind(family_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if family_revoked {
            let _ = sqlx::query(
                "UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(&claims.sub)
            .execute(pool)
            .await;
            return Err(AuthError::unauthorized("token family revoked — all sessions terminated"));
        }
    }

    // Check if this exact token was already used (replay)
    let already_used: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM refresh_token_used WHERE token_hash = $1)",
    )
    .bind(&token_hash)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if already_used {
        if !family_id.is_empty() {
            let _ = sqlx::query(
                "INSERT INTO refresh_token_revoked_families (id, family_id) VALUES ($1, $2)
                 ON CONFLICT (family_id) DO NOTHING",
            )
            .bind(primitives::generate_id())
            .bind(family_id)
            .execute(pool)
            .await;

            let _ = sqlx::query(
                "UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
            )
            .bind(&claims.sub)
            .execute(pool)
            .await;
        }
        return Err(AuthError::unauthorized("refresh token reuse detected — session terminated"));
    }

    // Mark current token as used
    let _ = sqlx::query(
        "INSERT INTO refresh_token_used (id, token_hash, family_id) VALUES ($1, $2, $3)",
    )
    .bind(primitives::generate_id())
    .bind(&token_hash)
    .bind(family_id)
    .execute(pool)
    .await;

    let user = ctx.user_finder().find_or_fail(&claims.sub).await?;

    let new_family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
        Some(new_family_id),
    )
    .map_err(AuthError::internal)
}

pub async fn forgot_password<C: AuthContext>(
    ctx: &C,
    email: &str,
) -> Result<(), AuthError> {
    let user = ctx.user_finder().find_by_email(email).await?;

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
        .execute(ctx.pool())
        .await
        {
            Ok(_) => {
                let reset_url = format!(
                    "{}/reset-password?token={}",
                    ctx.config().app_url, plain_token,
                );
                if let Err(e) = ctx.mailer()
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

pub async fn reset_password<C: AuthContext>(
    ctx: &C,
    token: &str,
    new_password: &str,
) -> Result<(), AuthError> {
    let token_hash = primitives::sha256_hex(token);

    let email: String = sqlx::query_scalar(
        "SELECT email FROM password_resets
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()
         LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(ctx.pool())
    .await?
    .ok_or_else(|| AuthError::bad_request("invalid or expired token"))?;

    let hash = primitives::hash_password(new_password).map_err(AuthError::internal)?;

    let user = ctx.user_finder()
        .find_by_email(&email)
        .await?
        .ok_or_else(|| AuthError::not_found("user not found"))?;

    ctx.user_finder().update_user(&user.id, &[("password_hash", Some(&hash))]).await?;

    let _ = sqlx::query(
        "UPDATE password_resets SET used_at = NOW() WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .execute(ctx.pool())
    .await;

    Ok(())
}
