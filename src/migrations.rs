use sqlx::migrate::Migrator;
use sqlx::PgPool;

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    let migrator = Migrator::new(std::path::Path::new("./database/migrations"))
        .await?;
    migrator.run(pool).await?;
    Ok(())
}
