use axum::routing::post;
use axum::Router;

use crate::app::controllers::auth_controller;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(auth_controller::register))
        .route("/login", post(auth_controller::login))
        .route("/logout", post(auth_controller::logout))
        .route("/refresh", post(auth_controller::refresh))
        .route("/forgot-password", post(auth_controller::forgot_password))
        .route("/reset-password", post(auth_controller::reset_password))
}
