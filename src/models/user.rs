use api_core::crud::CrudService;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::PgPool;

use crate::db;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub roles: String,
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl CrudService for User {
    const TABLE: &'static str = "users";

    fn pool() -> &'static PgPool {
        db::pool()
    }
}

impl User {
    pub async fn find_by_email(email: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE email = $1")
            .bind(email.to_lowercase())
            .fetch_optional(db::pool())
            .await
    }

    pub async fn verify_email(id: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(db::pool())
        .await
    }
}
