use api_core::auth;
use api_core::crud::FieldValue;
use api_core::crud::CrudService;

use crate::app::models::User;
use crate::config::AppConfig;
use crate::error::{AppError, OrInternal};
use crate::storage;

pub async fn list() -> Result<Vec<User>, AppError> {
    User::all().await.or_internal()
}

pub async fn get_by_id(id: &str) -> Result<User, AppError> {
    User::find_or_fail(id).await.or_internal()
}

pub async fn create(
    email: &str,
    password: &str,
    name: &str,
    roles: &str,
) -> Result<User, AppError> {
    let hash = auth::hash_password(password)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    User::create(&[
        ("id", FieldValue::String(auth::generate_id())),
        ("email", FieldValue::String(email.to_lowercase())),
        ("password_hash", FieldValue::String(hash)),
        ("name", FieldValue::String(name.to_string())),
        ("roles", FieldValue::String(roles.to_string())),
    ])
    .await
    .or_internal()
}

pub async fn update(
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
        return User::find_or_fail(id).await.or_internal();
    }

    User::update(id, &fields).await.or_internal()
}

pub async fn delete(id: &str) -> Result<bool, AppError> {
    User::find_or_fail(id).await.or_internal()?;
    User::delete(id).await.or_internal()
}

pub async fn get_profile(user_id: &str) -> Result<User, AppError> {
    User::find_or_fail(user_id).await.or_internal()
}

pub async fn upload_avatar(
    config: &AppConfig,
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

    let url = storage::save_avatar(&config.storage_dir, user_id, mime, data)
        .await
        .map_err(|e| AppError::BadRequest(e))?;

    let old = User::find_or_fail(user_id).await.or_internal()?;

    if let Some(old_url) = old.avatar_url {
        storage::delete_avatar(&config.storage_dir, &old_url).await;
    }

    User::update(user_id, &[("avatar_url", FieldValue::String(url.clone()))])
        .await
        .or_internal()?;

    Ok(url)
}
