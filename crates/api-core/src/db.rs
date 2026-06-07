use std::sync::OnceLock;

use sqlx::PgPool;

static POOL: OnceLock<PgPool> = OnceLock::new();

pub fn init(p: PgPool) {
    POOL.set(p).ok();
}

pub fn pool() -> &'static PgPool {
    POOL.get().expect("database pool not initialized")
}
