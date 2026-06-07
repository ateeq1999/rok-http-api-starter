use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::auth;

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

impl User {
    pub async fn find_by_email(pool: &sqlx::PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE email = $1")
            .bind(email.to_lowercase())
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_pk(pool: &sqlx::PgPool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn all(pool: &sqlx::PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    pub async fn create_user(
        pool: &sqlx::PgPool,
        email: &str,
        password_hash: &str,
        name: &str,
    ) -> Result<Self, sqlx::Error> {
        let id = auth::generate_id();
        sqlx::query_as::<_, Self>(
            "INSERT INTO users (id, email, password_hash, name, roles) VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(id)
        .bind(email.to_lowercase())
        .bind(password_hash)
        .bind(name)
        .bind("user")
        .fetch_one(pool)
        .await
    }

    pub async fn update_by_pk(
        pool: &sqlx::PgPool,
        id: &str,
        email: Option<&str>,
        name: Option<&str>,
        roles: Option<&str>,
        password_hash: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "UPDATE users SET
                email = COALESCE($2, email),
                name = COALESCE($3, name),
                roles = COALESCE($4, roles),
                password_hash = COALESCE($5, password_hash),
                updated_at = NOW()
             WHERE id = $1
             RETURNING *",
        )
        .bind(id)
        .bind(email.map(|e| e.to_lowercase()))
        .bind(name)
        .bind(roles)
        .bind(password_hash)
        .fetch_one(pool)
        .await
    }

    pub async fn verify_email(
        pool: &sqlx::PgPool,
        id: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn delete_by_pk(pool: &sqlx::PgPool, id: &str) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
    }
}
