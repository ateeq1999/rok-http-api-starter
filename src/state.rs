use std::sync::Arc;

use rok_auth::axum::{HasAuth, HasPool};
use rok_auth::Auth;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub auth: Arc<Auth>,
    pub config: crate::config::AppConfig,
}

impl HasPool for AppState {
    fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

impl HasAuth for AppState {
    fn auth_handle(&self) -> Arc<Auth> {
        self.auth.clone()
    }
}
