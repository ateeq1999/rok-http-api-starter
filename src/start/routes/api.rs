use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::app::controllers::{rbac_controller, user_controller};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // User CRUD
        .route("/users", get(user_controller::index))
        .route("/users", post(user_controller::store))
        .route("/users/{id}", get(user_controller::show))
        .route("/users/{id}", put(user_controller::update))
        .route("/users/{id}", delete(user_controller::destroy))
        // Current user
        .route("/me", get(user_controller::me))
        .route("/me/avatar", post(user_controller::upload_avatar))
        .route("/me/permissions", get(rbac_controller::my_permissions))
        // RBAC management (admin)
        .route("/roles", get(rbac_controller::list_roles))
        .route("/roles", post(rbac_controller::create_role))
        .route("/roles/{id}", delete(rbac_controller::delete_role))
        .route("/roles/{id}/permissions", post(rbac_controller::grant_permission))
        .route("/roles/{id}/permissions/{perm_id}", delete(rbac_controller::revoke_permission))
        .route("/permissions", get(rbac_controller::list_permissions))
        .route("/users/{id}/roles", post(rbac_controller::assign_role))
        .route("/users/{id}/roles/{role_id}", delete(rbac_controller::remove_role))
}
