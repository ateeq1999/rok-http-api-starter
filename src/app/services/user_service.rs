use std::sync::Arc;

use api_core::crud::FieldValue;
use auth::primitives;
use di::injectable;

use crate::app::models::User;
use crate::app::repositories::{AvatarStorage, UserRepository};
use crate::error::{AppError, OrInternal};

#[injectable]
pub struct UserService {
    #[inject]
    users: Arc<dyn UserRepository>,
    #[inject]
    avatars: Arc<dyn AvatarStorage>,
}

impl UserService {
    pub async fn list(&self) -> Result<Vec<User>, AppError> {
        self.users.all().await.or_internal()
    }

    pub async fn get_by_id(&self, id: &str) -> Result<User, AppError> {
        self.users.find_or_fail(id).await.or_internal()
    }

    pub async fn create(
        &self,
        email: &str,
        password: &str,
        name: &str,
        roles: &str,
    ) -> Result<User, AppError> {
        let hash = primitives::hash_password(password)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.users
            .create(&[
                ("id", FieldValue::String(primitives::generate_id())),
                ("email", FieldValue::String(email.to_lowercase())),
                ("password_hash", FieldValue::String(hash)),
                ("name", FieldValue::String(name.to_string())),
                ("roles", FieldValue::String(roles.to_string())),
            ])
            .await
            .or_internal()
    }

    pub async fn update(
        &self,
        id: &str,
        email: Option<&str>,
        name: Option<&str>,
        roles: Option<&str>,
    ) -> Result<User, AppError> {
        let mut fields: Vec<(&str, FieldValue)> = Vec::new();
        if let Some(email) = email {
            fields.push(("email", FieldValue::String(email.to_lowercase())));
        }
        if let Some(name) = name {
            fields.push(("name", FieldValue::String(name.to_string())));
        }
        if let Some(roles) = roles {
            fields.push(("roles", FieldValue::String(roles.to_string())));
        }

        if fields.is_empty() {
            return self.users.find_or_fail(id).await.or_internal();
        }

        self.users.update(id, &fields).await.or_internal()
    }

    pub async fn delete(&self, id: &str) -> Result<bool, AppError> {
        self.users.find_or_fail(id).await.or_internal()?;
        self.users.delete(id).await.or_internal()
    }

    pub async fn get_profile(&self, user_id: &str) -> Result<User, AppError> {
        self.users.find_or_fail(user_id).await.or_internal()
    }

    pub async fn upload_avatar(
        &self,
        user_id: &str,
        mime: &str,
        data: &[u8],
    ) -> Result<String, AppError> {
        if data.is_empty() {
            return Err(AppError::BadRequest("empty file".into()));
        }

        if data.len() > 5 * 1024 * 1024 {
            return Err(AppError::BadRequest("file too large (max 5 MB)".into()));
        }

        let url = self
            .avatars
            .save_avatar(user_id, mime, data)
            .await
            .map_err(AppError::BadRequest)?;

        let old = self.users.find_or_fail(user_id).await.or_internal()?;

        if let Some(old_url) = old.avatar_url {
            self.avatars.delete_avatar(&old_url).await;
        }

        self.users
            .update(user_id, &[("avatar_url", FieldValue::String(url.clone()))])
            .await
            .or_internal()?;

        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    // Demonstrates the DI migration's testability goal: because `UserService` depends on the
    // `UserRepository`/`AvatarStorage` *interfaces* rather than concrete Postgres/filesystem
    // types, a unit test can construct it with hand-written fakes via the plain `new()` the
    // `#[injectable]` macro generates — no DI container, no database, involved at all.
    #[derive(Default)]
    struct FakeUserRepository {
        users: Mutex<Vec<User>>,
    }

    impl FakeUserRepository {
        fn field(fields: &[(&str, FieldValue)], key: &str) -> Option<String> {
            fields.iter().find(|(k, _)| *k == key).and_then(|(_, v)| match v {
                FieldValue::String(s) => Some(s.clone()),
                _ => None,
            })
        }
    }

    #[async_trait::async_trait]
    impl UserRepository for FakeUserRepository {
        async fn all(&self) -> Result<Vec<User>, sqlx::Error> {
            Ok(self.users.lock().unwrap().clone())
        }

        async fn find_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error> {
            Ok(self.users.lock().unwrap().iter().find(|u| u.id == id).cloned())
        }

        async fn find_or_fail(&self, id: &str) -> Result<User, sqlx::Error> {
            self.find_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)
        }

        async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
            Ok(self.users.lock().unwrap().iter().find(|u| u.email == email).cloned())
        }

        async fn find_by_identifier(&self, identifier: &str) -> Result<Option<User>, sqlx::Error> {
            self.find_by_email(identifier).await
        }

        async fn create(&self, fields: &[(&str, FieldValue)]) -> Result<User, sqlx::Error> {
            let user = User {
                id: Self::field(fields, "id").unwrap_or_default(),
                email: Self::field(fields, "email").unwrap_or_default(),
                password_hash: Self::field(fields, "password_hash").unwrap_or_default(),
                name: Self::field(fields, "name").unwrap_or_default(),
                roles: Self::field(fields, "roles").unwrap_or_else(|| "user".into()),
                username: None,
                avatar_url: None,
                email_verified_at: None,
                totp_secret: None,
                totp_enabled: false,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.users.lock().unwrap().push(user.clone());
            Ok(user)
        }

        async fn update(&self, id: &str, fields: &[(&str, FieldValue)]) -> Result<User, sqlx::Error> {
            let mut users = self.users.lock().unwrap();
            let user = users.iter_mut().find(|u| u.id == id).ok_or(sqlx::Error::RowNotFound)?;
            if let Some(v) = Self::field(fields, "avatar_url") {
                user.avatar_url = Some(v);
            }
            Ok(user.clone())
        }

        async fn delete(&self, id: &str) -> Result<bool, sqlx::Error> {
            let mut users = self.users.lock().unwrap();
            let len_before = users.len();
            users.retain(|u| u.id != id);
            Ok(users.len() != len_before)
        }
    }

    struct NoopAvatarStorage;

    #[async_trait::async_trait]
    impl AvatarStorage for NoopAvatarStorage {
        async fn save_avatar(&self, _user_id: &str, _mime: &str, _data: &[u8]) -> Result<String, String> {
            Ok("/storage/avatars/test.png".into())
        }
        async fn delete_avatar(&self, _url: &str) {}
    }

    fn service() -> UserService {
        UserService::new(Arc::new(FakeUserRepository::default()), Arc::new(NoopAvatarStorage))
    }

    #[tokio::test]
    async fn create_hashes_password_and_persists() {
        let service = service();
        let user = service.create("test@example.com", "supersecret", "Test User", "user").await.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_ne!(user.password_hash, "supersecret");

        let listed = service.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, user.id);
    }

    #[tokio::test]
    async fn upload_avatar_rejects_empty_file() {
        let service = service();
        let user = service.create("test@example.com", "supersecret", "Test User", "user").await.unwrap();
        let err = service.upload_avatar(&user.id, "image/png", &[]).await.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn upload_avatar_updates_user_record() {
        let service = service();
        let user = service.create("test@example.com", "supersecret", "Test User", "user").await.unwrap();
        let url = service.upload_avatar(&user.id, "image/png", &[0u8; 4]).await.unwrap();
        let refreshed = service.get_by_id(&user.id).await.unwrap();
        assert_eq!(refreshed.avatar_url, Some(url));
    }
}
