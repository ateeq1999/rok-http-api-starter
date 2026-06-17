use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

use crate::state::AppState;
use ::auth::middleware::JwtAuthLayer;

pub mod api;
pub mod auth;

/// Build the full router with JWT middleware applied to protected groups.
pub fn app_router(secret: &str) -> Router<AppState> {
  let jwt_layer = JwtAuthLayer::new(secret.to_string());

  Router::new()
    .route("/api/v1/health", get(api_core::health::get))
    // Public routes (no JWT)
    .nest("/api/v1/auth", auth::public_routes())
    // Protected routes (JWT required)
    .nest(
      "/api/v1/auth",
      auth::protected_routes().layer(jwt_layer.clone()),
    )
    .nest("/api/v1", api::routes().layer(jwt_layer))
    .nest_service("/storage", ServeDir::new("./storage"))
}
