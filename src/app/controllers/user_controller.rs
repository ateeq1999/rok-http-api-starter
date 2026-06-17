use axum::extract::Multipart;
use axum::extract::Path;
use axum::extract::State;
use axum::Json;

use api_core::crud::FieldValue;
use api_core::crud::CrudService;
use api_core::response::{ApiResponse, ErrorCode};

use crate::auth::AdminOnly;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::app::models::User;
use crate::state::AppState;
use crate::storage;
use crate::app::validators;
use crate::app::validators::user::*;

pub async fn index(_admin: AdminOnly) -> Result<ApiResponse, AppError> {
    let users = User::all().await?;
    Ok(ApiResponse::ok(serde_json::json!({ "users": users })))
}

pub async fn show(
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> Result<ApiResponse, AppError> {
    let user = User::find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}

pub async fn store(
    _admin: AdminOnly,
    Json(body): Json<CreateUserRequest>,
) -> Result<ApiResponse, validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let hash = match api_core::auth::hash_password(&body.password) {
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(h) => h,
    };

    match User::create(&[
        ("id", FieldValue::String(api_core::auth::generate_id())),
        ("email", FieldValue::String(body.email.to_lowercase())),
        ("password_hash", FieldValue::String(hash)),
        ("name", FieldValue::String(body.name)),
        ("roles", FieldValue::String(body.roles)),
    ])
    .await
    {
        Err(e) => Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(user) => Ok(ApiResponse::created(serde_json::json!({ "user": user }))),
    }
}

pub async fn update(
    _admin: AdminOnly,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<ApiResponse, validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let exists = User::find_by_id(&id).await.unwrap_or(None).is_some();

    if !exists {
        return Ok(ApiResponse::error(ErrorCode::NotFound, "user not found"));
    }

    let mut fields: Vec<(&str, FieldValue)> = Vec::new();
    if let Some(email) = &body.email {
        fields.push(("email", FieldValue::String(email.to_lowercase())));
    }
    if let Some(name) = &body.name {
        fields.push(("name", FieldValue::String(name.clone())));
    }
    if let Some(roles) = &body.roles {
        fields.push(("roles", FieldValue::String(roles.clone())));
    }

    if !fields.is_empty() {
        match User::update(&id, &fields).await {
            Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
            Ok(_) => {}
        }
    }

    Ok(ApiResponse::ok(serde_json::json!({ "message": "updated" })))
}

pub async fn destroy(
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> ApiResponse {
    match User::find_by_id(&id).await {
        Err(e) => ApiResponse::error(ErrorCode::InternalServerError, e.to_string()),
        Ok(None) => ApiResponse::error(ErrorCode::NotFound, "user not found"),
        Ok(Some(_)) => match User::delete(&id).await {
            Err(e) => ApiResponse::error(ErrorCode::InternalServerError, e.to_string()),
            Ok(true) => ApiResponse::no_content(),
            Ok(false) => ApiResponse::error(ErrorCode::NotFound, "user not found"),
        },
    }
}

pub async fn me(user: AuthUser) -> Result<ApiResponse, AppError> {
    let user = User::find_by_id(&user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<ApiResponse, AppError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("invalid multipart data".into()))?
        .ok_or_else(|| AppError::BadRequest("no file uploaded".into()))?;

    let mime = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field
        .bytes()
        .await
        .map_err(|_| AppError::BadRequest("failed to read file data".into()))?;

    if data.is_empty() {
        return Err(AppError::BadRequest("empty file".into()));
    }

    if data.len() > 5 * 1024 * 1024 {
        return Err(AppError::BadRequest("file too large (max 5 MB)".into()));
    }

    let url = storage::save_avatar(
        &state.config.storage_dir,
        &user.user_id,
        &mime,
        &data,
    )
    .await
    .map_err(|e| AppError::BadRequest(e))?;

    let old: User = User::find_by_id(&user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    if let Some(old_url) = old.avatar_url {
        storage::delete_avatar(&state.config.storage_dir, &old_url).await;
    }

    User::update(&user.user_id, &[("avatar_url", FieldValue::String(url.clone()))])
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(ApiResponse::ok(serde_json::json!({ "avatar_url": url })))
}
