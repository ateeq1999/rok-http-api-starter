use rok_orm::{FileSource, MigrationRunner};
use sqlx::PgPool;

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    MigrationRunner::new(pool.clone())
        .source(FileSource::new("./database/migrations"))
        .run()
        .await
}
