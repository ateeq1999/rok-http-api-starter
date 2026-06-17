use axum::routing::post;
use axum::Router;

use crate::app::controllers::auth_controller;
use crate::state::AppState;

/// Public auth routes (no JWT required).
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(auth_controller::register))
        .route("/login", post(auth_controller::login))
        .route("/forgot-password", post(auth_controller::forgot_password))
        .route("/reset-password", post(auth_controller::reset_password))
}

/// Protected auth routes (JWT required).
pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/logout", post(auth_controller::logout))
        .route("/refresh", post(auth_controller::refresh))
}
