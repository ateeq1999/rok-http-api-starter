use std::collections::HashSet;
use std::path::Path;

use sqlx::migrate::Migrate;
use sqlx::migrate::Migrator;
use sqlx::PgPool;

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    let migrator = Migrator::new(Path::new("./database/migrations")).await?;
    migrator.run(pool).await?;
    Ok(())
}

pub async fn rollback(pool: &PgPool) -> anyhow::Result<()> {
    let migrator = Migrator::new(Path::new("./database/migrations")).await?;

    let has_down = migrator.iter().any(|m| m.migration_type.is_down_migration());
    if !has_down {
        println!("No down-migration files found (*.down.sql).");
        println!("Rollback requires reversible migrations with .up.sql / .down.sql pairs.");
        return Ok(());
    }

    let mut conn = pool.acquire().await?;
    let conn = &mut *conn;
    let applied = conn.list_applied_migrations().await?;
    let count = applied.len();
    if count == 0 {
        println!("No migrations to roll back");
        return Ok(());
    }

    migrator.undo(pool, 0).await?;
    println!("Rolled back 1 migration");
    Ok(())
}

pub async fn status(pool: &PgPool) -> anyhow::Result<()> {
    let migrator = Migrator::new(Path::new("./database/migrations")).await?;
    let mut conn = pool.acquire().await?;
    let conn = &mut *conn;

    conn.ensure_migrations_table().await?;
    let applied = conn.list_applied_migrations().await?;
    let applied_versions: HashSet<i64> = applied.iter().map(|m| m.version).collect();

    let mut total = 0;
    for migration in migrator.iter() {
        if !migration.migration_type.is_up_migration() {
            continue;
        }
        total += 1;
        let mark = if applied_versions.contains(&migration.version) {
            "✓"
        } else {
            " "
        };
        println!("{} {:>3}  {}", mark, migration.version, migration.description);
    }

    if total == 0 {
        println!("No migrations found");
        return Ok(());
    }

    let pending = migrator
        .iter()
        .filter(|m| m.migration_type.is_up_migration())
        .filter(|m| !applied_versions.contains(&m.version))
        .count();
    let applied_count = total - pending;
    println!("\n{applied_count} applied, {pending} pending");
    Ok(())
}
