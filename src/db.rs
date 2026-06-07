use std::sync::OnceLock;

use sqlx::PgPool;

static POOL: OnceLock<PgPool> = OnceLock::new();

pub fn init(pool: PgPool) {
    POOL.set(pool).expect("DB already initialized");
}

pub fn pool() -> &'static PgPool {
    POOL.get().expect("DB not initialized")
}
