use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::app::controllers::{otp_controller, session_controller, user_controller};
use crate::state::AppState;

/// Protected API routes (JWT required).
pub fn routes() -> Router<AppState> {
    Router::new()
        // Users (admin)
        .route("/users", get(user_controller::index))
        .route("/users", post(user_controller::store))
        .route("/users/{id}", get(user_controller::show))
        .route("/users/{id}", put(user_controller::update))
        .route("/users/{id}", delete(user_controller::destroy))
        // Profile
        .route("/me", get(user_controller::me))
        .route("/me/avatar", post(user_controller::upload_avatar))
        // Sessions
        .route("/me/sessions", get(session_controller::list))
        .route("/me/sessions/{id}", delete(session_controller::revoke))
        .route("/me/sessions", delete(session_controller::revoke_all))
        // OTP (registration verification)
        .route("/otp/send", post(otp_controller::send))
        .route("/otp/verify", post(otp_controller::verify))
}
