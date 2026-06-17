use crate::context::AuthContext;
use crate::error::AuthError;
use crate::primitives;
use crate::session::Session;

pub async fn create<C: AuthContext>(
    ctx: &C,
    user_id: &str,
    access_token: &str,
    device_info: Option<&str>,
    ip_address: Option<&str>,
) -> Result<Session, AuthError> {
    let id = primitives::generate_id();
    let access_token_hash = primitives::sha256_hex(access_token);

    sqlx::query_as::<_, Session>(
        "INSERT INTO sessions (id, user_id, device_info, ip_address, access_token_hash)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(&id)
    .bind(user_id)
    .bind(device_info)
    .bind(ip_address)
    .bind(&access_token_hash)
    .fetch_one(ctx.pool())
    .await
    .map_err(AuthError::from)
}

pub async fn find_valid_by_token<C: AuthContext>(
    ctx: &C,
    access_token: &str,
) -> Result<Option<Session>, AuthError> {
    let hash = primitives::sha256_hex(access_token);
    sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE access_token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&hash)
    .fetch_optional(ctx.pool())
    .await
    .map_err(AuthError::from)
}

pub async fn revoke<C: AuthContext>(ctx: &C, session_id: &str) -> Result<(), AuthError> {
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(ctx.pool())
        .await?;
    Ok(())
}

pub async fn revoke_all_for_user<C: AuthContext>(ctx: &C, user_id: &str) -> Result<(), AuthError> {
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(ctx.pool())
        .await?;
    Ok(())
}

pub async fn list_for_user<C: AuthContext>(ctx: &C, user_id: &str) -> Result<Vec<Session>, AuthError> {
    sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE user_id = $1 AND revoked_at IS NULL ORDER BY last_seen_at DESC",
    )
    .bind(user_id)
    .fetch_all(ctx.pool())
    .await
    .map_err(AuthError::from)
}
