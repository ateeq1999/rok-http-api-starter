use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::auth::AdminOnly;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::User;
use crate::response;
use crate::state::AppState;
use crate::validators;
use crate::validators::user::*;

pub async fn index(
    State(state): State<AppState>,
    _admin: AdminOnly,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let users = User::all(&state.pool).await?;
    Ok(response::ok(serde_json::json!({ "users": users })))
}

pub async fn show(
    State(state): State<AppState>,
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let user = User::find_by_pk(&state.pool, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;
    Ok(response::ok(serde_json::json!({ "user": user })))
}

pub async fn store(
    State(state): State<AppState>,
    _admin: AdminOnly,
    Json(body): Json<CreateUserRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let hash = match crate::auth::hash_password(&body.password) {
        Err(e) => return Ok(response::error("E_HASH", &e.to_string(), 500)),
        Ok(h) => h,
    };

    match User::create_user(&state.pool, &body.email, &hash, &body.name).await {
        Err(e) => Ok(response::error("E_CREATE", &e.to_string(), 500)),
        Ok(user) => Ok(response::created(serde_json::json!({ "user": user }))),
    }
}

pub async fn update(
    State(state): State<AppState>,
    _admin: AdminOnly,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> Result<(axum::http::StatusCode, Json<Value>), validators::ValidationRejection> {
    let body = validators::validate(body)?;

    let user = User::find_by_pk(&state.pool, &id)
        .await
        .unwrap_or(None);

    if user.is_none() {
        return Ok(response::error("E_ROW_NOT_FOUND", "user not found", 404));
    }

    match User::update_by_pk(
        &state.pool,
        &id,
        body.email.as_deref(),
        body.name.as_deref(),
        body.roles.as_deref(),
        None,
    )
    .await
    {
        Err(e) => Ok(response::error("E_UPDATE", &e.to_string(), 500)),
        Ok(_) => Ok(response::ok(serde_json::json!({ "message": "updated" }))),
    }
}

pub async fn destroy(
    State(state): State<AppState>,
    _admin: AdminOnly,
    Path(id): Path<String>,
) -> (axum::http::StatusCode, Json<Value>) {
    let user = User::find_by_pk(&state.pool, &id).await;

    match user {
        Err(e) => response::error("E_DATABASE", &e.to_string(), 500),
        Ok(None) => response::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(_)) => match User::delete_by_pk(&state.pool, &id).await {
            Err(e) => response::error("E_DELETE", &e.to_string(), 500),
            Ok(_) => {
                let status = response::no_content();
                (status, Json(serde_json::json!({})))
            }
        },
    }
}

pub async fn me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<(axum::http::StatusCode, Json<Value>), AppError> {
    let user = User::find_by_pk(&state.pool, &user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;
    Ok(response::ok(serde_json::json!({ "user": user })))
}
