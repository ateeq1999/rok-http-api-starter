use std::sync::Arc;

use api_core::crud::FieldValue;
use di::injectable;

use super::user_repository::UserRepository;
use crate::app::models::User;

/// Bridges the DI-registered [`UserRepository`] into `crates/auth`'s `AuthContext` shape, which
/// needs an owned `Arc<dyn UserFinder>` cached on `AppState` (see `src/state.rs`).
#[injectable]
pub struct AppUserFinder {
    #[inject]
    users: Arc<dyn UserRepository>,
}

#[async_trait::async_trait]
impl auth::context::UserFinder for AppUserFinder {
    async fn find_by_email(&self, email: &str) -> Result<Option<auth::context::UserRecord>, sqlx::Error> {
        Ok(self.users.find_by_email(email).await?.map(auth::context::UserRecord::from))
    }

    async fn find_by_identifier(&self, identifier: &str) -> Result<Option<auth::context::UserRecord>, sqlx::Error> {
        Ok(self.users.find_by_identifier(identifier).await?.map(auth::context::UserRecord::from))
    }

    async fn find_or_fail(&self, id: &str) -> Result<auth::context::UserRecord, sqlx::Error> {
        self.users.find_or_fail(id).await.map(auth::context::UserRecord::from)
    }

    async fn create_user(&self, fields: &[(&str, &str)]) -> Result<auth::context::UserRecord, sqlx::Error> {
        let fv_fields: Vec<(&str, FieldValue)> = fields
            .iter()
            .map(|(k, v)| (*k, FieldValue::String(v.to_string())))
            .collect();
        self.users.create(&fv_fields).await.map(auth::context::UserRecord::from)
    }

    async fn update_user(&self, id: &str, fields: &[(&str, Option<&str>)]) -> Result<auth::context::UserRecord, sqlx::Error> {
        let fv_fields: Vec<(&str, FieldValue)> = fields
            .iter()
            .map(|(k, v)| {
                (
                    *k,
                    match v {
                        Some(val) => FieldValue::String(val.to_string()),
                        None => FieldValue::OptionString(None),
                    },
                )
            })
            .collect();
        self.users.update(id, &fv_fields).await.map(auth::context::UserRecord::from)
    }
}

impl From<User> for auth::context::UserRecord {
    fn from(u: User) -> Self {
        auth::context::UserRecord {
            id: u.id,
            email: u.email,
            password_hash: u.password_hash,
            name: u.name,
            roles: u.roles,
            username: u.username,
            avatar_url: u.avatar_url,
            email_verified_at: u.email_verified_at,
            totp_secret: u.totp_secret,
            totp_enabled: u.totp_enabled,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}
