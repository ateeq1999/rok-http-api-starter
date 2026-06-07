use axum::extract::Path;
use axum::Json;
use serde_json::Value;

use crate::auth::AdminOnly;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::User;
use crate::response;
use crate::services::crud::{CrudService, FieldValue};
use crate::validators;
use crate::validators::user::*;

pub async fn index(
    _admin: AdminOnly,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let users = User::all().await?;
    Ok(response::ok(serde_json::json!({ "users": users })))
}

pub async fn show(
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let user = User::find_or_fail(&id).await?;
    Ok(response::ok(serde_json::json!({ "user": user })))
}

pub async fn store(
    _admin: AdminOnly,
    Json(body): Json<CreateUserRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let hash = match crate::auth::hash_password(&body.password) {
        Err(e) => return Ok(response::error("E_HASH", &e.to_string(), 500)),
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
        Err(e) => Ok(response::error("E_CREATE", &e.to_string(), 500)),
        Ok(user) => Ok(response::created(serde_json::json!({ "user": user }))),
    }
}

pub async fn update(
    _admin: AdminOnly,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let exists = User::find_by_id(&id).await.unwrap_or(None).is_some();

    if !exists {
        return Ok(response::error("E_ROW_NOT_FOUND", "user not found", 404));
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
            Err(e) => return Ok(response::error("E_UPDATE", &e.to_string(), 500)),
            Ok(_) => {}
        }
    }

    Ok(response::ok(serde_json::json!({ "message": "updated" })))
}

pub async fn destroy(
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> (axum::http::StatusCode, Json<Value>) {
    match User::find_by_id(&id).await {
        Err(e) => response::error("E_DATABASE", &e.to_string(), 500),
        Ok(None) => response::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(_)) => match User::delete(&id).await {
            Err(e) => response::error("E_DELETE", &e.to_string(), 500),
            Ok(true) => {
                let status = response::no_content();
                (status, Json(serde_json::json!({})))
            }
            Ok(false) => response::error("E_ROW_NOT_FOUND", "user not found", 404),
        },
    }
}

pub async fn me(user: AuthUser) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let user = User::find_or_fail(&user.user_id).await?;
    Ok(response::ok(serde_json::json!({ "user": user })))
}
