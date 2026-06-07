use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::auth;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailVerificationToken {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl EmailVerificationToken {
    pub async fn create(
        pool: &sqlx::PgPool,
        user_id: &str,
        token_hash: &str,
        expires_at: &chrono::DateTime<chrono::Utc>,
    ) -> Result<Self, sqlx::Error> {
        let id = auth::generate_id();
        sqlx::query_as::<_, Self>(
            "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    }

    pub async fn find_valid(
        pool: &sqlx::PgPool,
        user_id: &str,
        token_hash: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM email_verification_tokens
             WHERE user_id = $1
               AND token_hash = $2
               AND used_at IS NULL
               AND expires_at > NOW()
             LIMIT 1",
        )
        .bind(user_id)
        .bind(token_hash)
        .fetch_optional(pool)
        .await
    }

    pub async fn mark_used(
        pool: &sqlx::PgPool,
        id: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "UPDATE email_verification_tokens SET used_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn invalidate_previous(
        pool: &sqlx::PgPool,
        user_id: &str,
    ) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query(
            "UPDATE email_verification_tokens SET used_at = NOW()
             WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .execute(pool)
        .await
    }
}
