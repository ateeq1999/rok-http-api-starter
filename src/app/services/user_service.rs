use api_core::auth;
use api_core::crud::FieldValue;
use api_core::crud::CrudService;

use crate::app::models::User;
use crate::config::AppConfig;
use crate::error::AppError;
use crate::storage;

pub async fn list() -> Result<Vec<User>, AppError> {
    User::all().await.map_err(|e| AppError::Database(e.to_string()))
}

pub async fn get_by_id(id: &str) -> Result<User, AppError> {
    User::find_by_id(id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("user not found".into()))
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
    .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn update(
    id: &str,
    email: Option<&str>,
    name: Option<&str>,
    roles: Option<&str>,
) -> Result<User, AppError> {
    let exists = User::find_by_id(id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .is_some();

    if !exists {
        return Err(AppError::NotFound("user not found".into()));
    }

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
        return User::find_by_id(id)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("user not found".into()));
    }

    User::update(id, &fields)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn delete(id: &str) -> Result<bool, AppError> {
    let exists = User::find_by_id(id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .is_some();

    if !exists {
        return Err(AppError::NotFound("user not found".into()));
    }

    User::delete(id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))
}

pub async fn get_profile(user_id: &str) -> Result<User, AppError> {
    User::find_by_id(user_id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("user not found".into()))
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

    let old: User = User::find_by_id(user_id)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    if let Some(old_url) = old.avatar_url {
        storage::delete_avatar(&config.storage_dir, &old_url).await;
    }

    User::update(user_id, &[("avatar_url", FieldValue::String(url.clone()))])
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(url)
}
