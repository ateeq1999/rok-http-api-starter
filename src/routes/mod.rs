use axum::Router;

use crate::state::AppState;

pub mod auth;
pub mod api;

pub fn app_router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/api/v1", api::routes())
}
