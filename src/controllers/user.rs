use axum::extract::Path;

use rok_auth::axum::RequestContext;
use rok_auth::axum::RequireRole;
use rok_core::api::ApiResponse;
use rok_orm::PgModel;
use rok_validate::Valid;

use crate::error::AppError;
use crate::guards::Admin;
use crate::models::User;
use crate::validators::user::*;

pub async fn index(
    _ctx: RequestContext,
    _: RequireRole<Admin>,
) -> Result<ApiResponse, AppError> {
    let users = User::all().await?;
    Ok(ApiResponse::ok(serde_json::json!({ "users": users })))
}

pub async fn show(
    _ctx: RequestContext,
    _: RequireRole<Admin>,
    Path(id): Path<i64>,
) -> Result<ApiResponse, AppError> {
    let user = match User::find_by_pk(id).await? {
        Some(u) => u,
        None => return Err(AppError::NotFound("user not found".into())),
    };
    Ok(ApiResponse::ok(serde_json::json!({ "user": user })))
}

pub async fn store(
    ctx: RequestContext,
    _: RequireRole<Admin>,
    Valid(body): Valid<CreateUserRequest>,
) -> ApiResponse {
    let hash = match rok_auth::password::hash_async(body.password.clone()).await {
        Err(e) => return ApiResponse::error("E_HASH", e.to_string(), 500),
        Ok(h) => h,
    };

    match User::create_user(ctx.db(), &body.email, &hash, &body.name).await {
        Err(e) => ApiResponse::error("E_CREATE", e.to_string(), 500),
        Ok(user) => ApiResponse::created(serde_json::json!({ "user": user })),
    }
}

pub async fn update(
    _ctx: RequestContext,
    _: RequireRole<Admin>,
    Path(id): Path<i64>,
    Valid(body): Valid<UpdateUserRequest>,
) -> ApiResponse {
    if User::find_by_pk(id).await.map(|u| u.is_none()).unwrap_or(true) {
        return ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404);
    }

    let mut updates = Vec::new();
    if let Some(email) = &body.email {
        updates.push(("email", rok_orm::SqlValue::Text(email.clone())));
    }
    if let Some(name) = &body.name {
        updates.push(("name", rok_orm::SqlValue::Text(name.clone())));
    }
    if let Some(roles) = &body.roles {
        updates.push(("roles", rok_orm::SqlValue::Text(roles.clone())));
    }

    if !updates.is_empty() {
        match User::update_by_pk(id, &updates).await {
            Err(e) => return ApiResponse::error("E_UPDATE", e.to_string(), 500),
            Ok(_) => {}
        }
    }

    ApiResponse::ok(serde_json::json!({ "message": "updated" }))
}

pub async fn destroy(
    _ctx: RequestContext,
    _: RequireRole<Admin>,
    Path(id): Path<i64>,
) -> ApiResponse {
    match User::find_by_pk(id).await {
        Err(e) => return ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => return ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(_)) => {
            match User::delete_by_pk(id).await {
                Err(e) => ApiResponse::error("E_DELETE", e.to_string(), 500),
                Ok(_) => ApiResponse::no_content(),
            }
        }
    }
}

pub async fn me(
    _ctx: RequestContext,
    claims: rok_auth::Claims,
) -> ApiResponse {
    let id: i64 = match claims.sub.parse() {
        Err(_) => return ApiResponse::error("E_INVALID_TOKEN", "invalid user id in token", 500),
        Ok(id) => id,
    };

    match User::find_by_pk(id).await {
        Err(e) => ApiResponse::error("E_DATABASE", e.to_string(), 500),
        Ok(None) => ApiResponse::error("E_ROW_NOT_FOUND", "user not found", 404),
        Ok(Some(user)) => ApiResponse::ok(serde_json::json!({ "user": user })),
    }
}
