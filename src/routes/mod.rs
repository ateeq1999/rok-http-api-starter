use axum::Router;

use crate::state::AppState;

pub mod api;
pub mod auth;

pub fn app_router() -> Router<AppState> {
    Router::new()
        .nest("/api/v1/auth", auth::routes())
        .nest("/api/v1", api::routes())
}
