use std::sync::Arc;

use api_core::crud::{CrudService, FieldValue};
use di::injectable;
use sqlx::PgPool;

use crate::app::models::User;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn all(&self) -> Result<Vec<User>, sqlx::Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error>;
    async fn find_or_fail(&self, id: &str) -> Result<User, sqlx::Error>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error>;
    async fn find_by_identifier(&self, identifier: &str) -> Result<Option<User>, sqlx::Error>;
    async fn create(&self, fields: &[(&str, FieldValue)]) -> Result<User, sqlx::Error>;
    async fn update(&self, id: &str, fields: &[(&str, FieldValue)]) -> Result<User, sqlx::Error>;
    async fn delete(&self, id: &str) -> Result<bool, sqlx::Error>;
}

#[injectable]
pub struct PgUserRepository {
    #[inject]
    pool: Arc<PgPool>,
}

#[async_trait::async_trait]
impl UserRepository for PgUserRepository {
    async fn all(&self) -> Result<Vec<User>, sqlx::Error> {
        User::all(&self.pool).await
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error> {
        User::find_by_id(&self.pool, id).await
    }

    async fn find_or_fail(&self, id: &str) -> Result<User, sqlx::Error> {
        User::find_or_fail(&self.pool, id).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        User::find_by_email(&self.pool, email).await
    }

    async fn find_by_identifier(&self, identifier: &str) -> Result<Option<User>, sqlx::Error> {
        User::find_by_identifier(&self.pool, identifier).await
    }

    async fn create(&self, fields: &[(&str, FieldValue)]) -> Result<User, sqlx::Error> {
        User::create(&self.pool, fields).await
    }

    async fn update(&self, id: &str, fields: &[(&str, FieldValue)]) -> Result<User, sqlx::Error> {
        User::update(&self.pool, id, fields).await
    }

    async fn delete(&self, id: &str) -> Result<bool, sqlx::Error> {
        User::delete(&self.pool, id).await
    }
}
