use axum::extract::Multipart;
use axum::extract::Path;
use axum::extract::State;

use api_core::response::ApiResponse;

use crate::auth::AdminOnly;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::app::services;
use crate::state::AppState;
use crate::app::validators::ValidatedJson;
use crate::app::validators::user::*;

pub async fn index(_admin: AdminOnly) -> Result<ApiResponse, AppError> {
    let users = services::user_service::list().await?;
    Ok(ApiResponse::ok(serde_json::json!({ "users": users })))
}

pub async fn show(
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> Result<ApiResponse, AppError> {
    let user = services::user_service::get_by_id(&id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}

pub async fn store(
    _admin: AdminOnly,
    ValidatedJson(body): ValidatedJson<CreateUserRequest>,
) -> Result<ApiResponse, AppError> {
    let user = services::user_service::create(
        &body.email,
        &body.password,
        &body.name,
        &body.roles,
    )
    .await?;
    Ok(ApiResponse::created(serde_json::json!({ "user": user })))
}

pub async fn update(
    _admin: AdminOnly,
    Path(id): Path<String>,
    ValidatedJson(body): ValidatedJson<UpdateUserRequest>,
) -> Result<ApiResponse, AppError> {
    services::user_service::update(
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
    Path(id): Path<String>,
) -> Result<ApiResponse, AppError> {
    services::user_service::delete(&id).await?;
    Ok(ApiResponse::no_content())
}

pub async fn me(user: AuthUser) -> Result<ApiResponse, AppError> {
    let user = services::user_service::get_profile(&user.user_id).await?;
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

    let url = services::user_service::upload_avatar(&state.config, &user.user_id, &mime, &data).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "avatar_url": url })))
}
