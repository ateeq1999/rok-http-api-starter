use axum::extract::Path;
use axum::Json;

use crate::auth::AdminOnly;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::User;
use crate::response::{ApiResponse, ErrorCode};
use crate::services::crud::{CrudService, FieldValue};
use crate::validators;
use crate::validators::user::*;

pub async fn index(_admin: AdminOnly) -> Result<ApiResponse, AppError> {
    let users = User::all().await?;
    Ok(ApiResponse::ok(serde_json::json!({ "users": users })))
}

pub async fn show(
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> Result<ApiResponse, AppError> {
    let user = User::find_or_fail(&id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}

pub async fn store(
    _admin: AdminOnly,
    Json(body): Json<CreateUserRequest>,
) -> Result<ApiResponse, validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let hash = match crate::auth::hash_password(&body.password) {
        Err(e) => return Ok(ApiResponse::error(ErrorCode::InternalServerError, e.to_string())),
        Ok(h) => h,
    };

    match User::create(&[
        ("id", FieldValue::String(crate::auth::generate_id())),
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
    let user = User::find_or_fail(&user.user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}
