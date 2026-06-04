use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::controllers::{otp, user};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(user::index))
        .route("/users", post(user::store))
        .route("/users/{id}", get(user::show))
        .route("/users/{id}", put(user::update))
        .route("/users/{id}", delete(user::destroy))
        .route("/me", get(user::me))
        .route("/otp/send", post(otp::send))
        .route("/otp/verify", post(otp::verify))
}
