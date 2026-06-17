use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::app::controllers::{otp_controller, user_controller};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(user_controller::index))
        .route("/users", post(user_controller::store))
        .route("/users/{id}", get(user_controller::show))
        .route("/users/{id}", put(user_controller::update))
        .route("/users/{id}", delete(user_controller::destroy))
        .route("/me", get(user_controller::me))
        .route("/me/avatar", post(user_controller::upload_avatar))
        .route("/otp/send", post(otp_controller::send))
        .route("/otp/verify", post(otp_controller::verify))
}
