use async_trait::async_trait;
use rok_orm::{Migration, MigrationRunner, SchemaExecutor};
use sqlx::PgPool;

pub struct CreateUsersTable;

#[async_trait]
impl Migration for CreateUsersTable {
    fn name(&self) -> &str {
        "2026_06_04_000001_create_users_table"
    }

    async fn up(&self, schema: &SchemaExecutor) -> anyhow::Result<()> {
        schema
            .create("users", |t| {
                t.id();
                t.string("email").not_null().unique();
                t.string("password_hash").not_null();
                t.string("name").not_null();
                t.string("roles").not_null().default("'user'");
                t.timestamp("email_verified_at").nullable();
                t.timestamps();
            })
            .await
    }

    async fn down(&self, schema: &SchemaExecutor) -> anyhow::Result<()> {
        schema.drop_table_if_exists("users").await
    }
}

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    MigrationRunner::new(pool.clone())
        .source(rok_auth::migrations())
        .migration(CreateUsersTable)
        .run()
        .await
}
