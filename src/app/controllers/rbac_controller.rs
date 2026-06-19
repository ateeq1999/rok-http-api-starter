use axum::extract::{Path, State};

use crate::error::AppError;
use crate::state::AppState;
use api_core::response::ApiResponse;

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

pub async fn list_roles(
    State(state): State<AppState>,
) -> Result<ApiResponse, AppError> {
    let roles = auth::services::rbac_service::list_roles(&state).await?;
    Ok(ApiResponse::ok(serde_json::to_value(roles).unwrap()))
}

pub async fn create_role(
    State(state): State<AppState>,
    axum::Json(input): axum::Json<CreateRoleRequest>,
) -> Result<ApiResponse, AppError> {
    let role = auth::services::rbac_service::create_role(
        &state,
        &input.name,
        input.description.as_deref(),
    ).await?;
    Ok(ApiResponse::created(serde_json::to_value(role).unwrap()))
}

pub async fn delete_role(
    State(state): State<AppState>,
    Path(role_id): Path<String>,
) -> Result<ApiResponse, AppError> {
    auth::services::rbac_service::delete_role(&state, &role_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "role deleted" })))
}

pub async fn grant_permission(
    State(state): State<AppState>,
    Path(role_id): Path<String>,
    axum::Json(input): axum::Json<GrantPermissionRequest>,
) -> Result<ApiResponse, AppError> {
    auth::services::rbac_service::grant_permission_to_role(&state, &role_id, &input.permission_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "permission granted" })))
}

pub async fn revoke_permission(
    State(state): State<AppState>,
    Path((role_id, permission_id)): Path<(String, String)>,
) -> Result<ApiResponse, AppError> {
    auth::services::rbac_service::revoke_permission_from_role(&state, &role_id, &permission_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "permission revoked" })))
}

pub async fn list_permissions(
    State(state): State<AppState>,
) -> Result<ApiResponse, AppError> {
    let permissions = auth::services::rbac_service::list_permissions(&state).await?;
    Ok(ApiResponse::ok(serde_json::to_value(permissions).unwrap()))
}

pub async fn assign_role(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    axum::Json(input): axum::Json<AssignRoleRequest>,
) -> Result<ApiResponse, AppError> {
    auth::services::rbac_service::assign_role_to_user(&state, &user_id, &input.role_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "role assigned" })))
}

pub async fn remove_role(
    State(state): State<AppState>,
    Path((user_id, role_id)): Path<(String, String)>,
) -> Result<ApiResponse, AppError> {
    auth::services::rbac_service::remove_role_from_user(&state, &user_id, &role_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({ "message": "role removed" })))
}

pub async fn my_permissions(
    user: auth::extractors::AuthUser,
) -> Result<ApiResponse, AppError> {
    let permissions: Vec<&str> = user.claims.permissions.split(',').filter(|s| !s.is_empty()).collect();
    Ok(ApiResponse::ok(serde_json::to_value(permissions).unwrap()))
}
