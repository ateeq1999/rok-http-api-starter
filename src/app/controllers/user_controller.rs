use axum::extract::Multipart;
use axum::extract::Path;

use api_core::response::ApiResponse;

use auth::extractors::{AdminOnly, AuthUser};
use crate::error::AppError;
use crate::app::services::user_service::UserService;
use di::Injected;
use auth::validators::ValidatedJson;
use crate::app::validators::user::*;

pub async fn index(_admin: AdminOnly, Injected(users): Injected<UserService>) -> Result<ApiResponse, AppError> {
    let users = users.list().await?;
    Ok(ApiResponse::ok(serde_json::json!({ "users": users })))
}

pub async fn show(
    _admin: AdminOnly,
    Injected(users): Injected<UserService>,
    Path(id): Path<String>,
) -> Result<ApiResponse, AppError> {
    let user = users.get_by_id(&id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}

pub async fn store(
    _admin: AdminOnly,
    Injected(users): Injected<UserService>,
    ValidatedJson(body): ValidatedJson<CreateUserRequest>,
) -> Result<ApiResponse, AppError> {
    let user = users
        .create(&body.email, &body.password, &body.name, &body.roles)
        .await?;
    Ok(ApiResponse::created(serde_json::json!({ "user": user })))
}

pub async fn update(
    _admin: AdminOnly,
    Injected(users): Injected<UserService>,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<UpdateUserRequest>,
) -> Result<ApiResponse, AppError> {
    users
        .update(
            &id,
            body.email.as_deref(),
            body.name.as_deref(),
            body.roles.as_deref(),
        )
        .await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "updated" })))
}

pub async fn destroy(
    _admin: AdminOnly,
    Injected(users): Injected<UserService>,
    Path(id): Path<String>,
) -> Result<ApiResponse, AppError> {
    users.delete(&id).await?;
    Ok(ApiResponse::no_content())
}

pub async fn me(user: AuthUser, Injected(users): Injected<UserService>) -> Result<ApiResponse, AppError> {
    let user = users.get_profile(&user.user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}

pub async fn upload_avatar(
    Injected(users): Injected<UserService>,
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

    let url = users.upload_avatar(&user.user_id, &mime, &data).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "avatar_url": url })))
}
