use api_core::response::ApiResponse;
use axum::extract::{Path, State};

use crate::context::AuthContext;
use crate::error::AuthError;
use crate::extractors::AuthUser;
use crate::services;

#[derive(serde::Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct GrantPermissionRequest {
    pub permission_id: String,
}

#[derive(serde::Deserialize)]
pub struct AssignRoleRequest {
    pub role_id: String,
}

pub async fn list_roles<C: AuthContext>(
    State(ctx): State<C>,
) -> Result<ApiResponse, AuthError> {
    let roles = services::rbac_service::list_roles(&ctx).await?;
    Ok(ApiResponse::ok(serde_json::to_value(roles).unwrap()))
}

pub async fn create_role<C: AuthContext>(
    State(ctx): State<C>,
    axum::Json(input): axum::Json<CreateRoleRequest>,
) -> Result<ApiResponse, AuthError> {
    let role = services::rbac_service::create_role(&ctx, &input.name, input.description.as_deref()).await?;
    Ok(ApiResponse::created(serde_json::to_value(role).unwrap()))
}

pub async fn delete_role<C: AuthContext>(
    State(ctx): State<C>,
    Path(role_id): Path<String>,
) -> Result<ApiResponse, AuthError> {
    services::rbac_service::delete_role(&ctx, &role_id).await?;
    Ok(ApiResponse::message("role deleted"))
}

pub async fn grant_permission<C: AuthContext>(
    State(ctx): State<C>,
    Path(role_id): Path<String>,
    axum::Json(input): axum::Json<GrantPermissionRequest>,
) -> Result<ApiResponse, AuthError> {
    services::rbac_service::grant_permission_to_role(&ctx, &role_id, &input.permission_id).await?;
    Ok(ApiResponse::message("permission granted"))
}

pub async fn revoke_permission<C: AuthContext>(
    State(ctx): State<C>,
    Path((role_id, permission_id)): Path<(String, String)>,
) -> Result<ApiResponse, AuthError> {
    services::rbac_service::revoke_permission_from_role(&ctx, &role_id, &permission_id).await?;
    Ok(ApiResponse::message("permission revoked"))
}

pub async fn list_permissions<C: AuthContext>(
    State(ctx): State<C>,
) -> Result<ApiResponse, AuthError> {
    let permissions = services::rbac_service::list_permissions(&ctx).await?;
    Ok(ApiResponse::ok(serde_json::to_value(permissions).unwrap()))
}

pub async fn assign_role<C: AuthContext>(
    State(ctx): State<C>,
    Path(user_id): Path<String>,
    axum::Json(input): axum::Json<AssignRoleRequest>,
) -> Result<ApiResponse, AuthError> {
    services::rbac_service::assign_role_to_user(&ctx, &user_id, &input.role_id).await?;
    Ok(ApiResponse::message("role assigned"))
}

pub async fn remove_role<C: AuthContext>(
    State(ctx): State<C>,
    Path((user_id, role_id)): Path<(String, String)>,
) -> Result<ApiResponse, AuthError> {
    services::rbac_service::remove_role_from_user(&ctx, &user_id, &role_id).await?;
    Ok(ApiResponse::message("role removed"))
}

pub async fn my_permissions(
    user: AuthUser,
) -> Result<ApiResponse, AuthError> {
    let permissions: Vec<&str> = user.claims.permissions.split(',').filter(|s| !s.is_empty()).collect();
    Ok(ApiResponse::ok(serde_json::to_value(permissions).unwrap()))
}
