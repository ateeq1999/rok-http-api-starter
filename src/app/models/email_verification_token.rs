use api_core::crud::CrudService;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmailVerificationToken {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl CrudService for EmailVerificationToken {
    const TABLE: &'static str = "email_verification_tokens";
}

#[allow(dead_code)]
impl EmailVerificationToken {
    pub async fn find_valid(
        pool: &PgPool,
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

    pub async fn mark_used(pool: &PgPool, id: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "UPDATE email_verification_tokens SET used_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn invalidate_previous(
        pool: &PgPool,
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
