use api_core::db;
use auth::primitives;
use auth::session::Session;

use crate::error::{AppError, OrInternal};

pub async fn create(
    user_id: &str,
    access_token: &str,
    device_info: Option<&str>,
    ip_address: Option<&str>,
) -> Result<Session, AppError> {
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
    .fetch_one(db::pool())
    .await
    .or_internal()
}

pub async fn find_valid_by_token(access_token: &str) -> Result<Option<Session>, AppError> {
    let hash = primitives::sha256_hex(access_token);
    sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE access_token_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&hash)
    .fetch_optional(db::pool())
    .await
    .or_internal()
}

pub async fn revoke(session_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(db::pool())
        .await
        .or_internal()?;
    Ok(())
}

pub async fn revoke_all_for_user(user_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(db::pool())
        .await
        .or_internal()?;
    Ok(())
}

pub async fn touch(session_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE sessions SET last_seen_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(db::pool())
        .await
        .or_internal()?;
    Ok(())
}

pub async fn list_for_user(user_id: &str) -> Result<Vec<Session>, AppError> {
    sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions WHERE user_id = $1 AND revoked_at IS NULL ORDER BY last_seen_at DESC",
    )
    .bind(user_id)
    .fetch_all(db::pool())
    .await
    .or_internal()
}
