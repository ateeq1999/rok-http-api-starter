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

async fn ensure_down_files() -> anyhow::Result<()> {
    let migrator = Migrator::new(Path::new("./database/migrations")).await?;
    if !migrator.iter().any(|m| m.migration_type.is_down_migration()) {
        println!("No down-migration files found (*.down.sql).");
        println!("This command requires reversible migrations with .up.sql / .down.sql pairs.");
        std::process::exit(1);
    }
    Ok(())
}

pub async fn rollback(pool: &PgPool) -> anyhow::Result<()> {
    ensure_down_files().await?;

    let mut conn = pool.acquire().await?;
    let conn = &mut *conn;
    let applied = conn.list_applied_migrations().await?;
    let max = applied.iter().map(|m| m.version).max();
    match max {
        None => {
            println!("No migrations to roll back");
            Ok(())
        }
        Some(version) => {
            let migrator = Migrator::new(Path::new("./database/migrations")).await?;
            migrator.undo(pool, version - 1).await?;
            println!("Rolled back migration {version}");
            Ok(())
        }
    }
}

pub async fn fresh(pool: &PgPool) -> anyhow::Result<()> {
    ensure_down_files().await?;

    println!("Dropping all tables...");
    sqlx::query("DROP SCHEMA IF EXISTS public CASCADE")
        .execute(pool).await?;
    sqlx::query("CREATE SCHEMA public")
        .execute(pool).await?;

    println!("Running all migrations...");
    run(pool).await?;
    println!("Fresh complete");
    Ok(())
}

pub async fn refresh(pool: &PgPool) -> anyhow::Result<()> {
    ensure_down_files().await?;

    let mut conn = pool.acquire().await?;
    let conn2 = &mut *conn;
    conn2.ensure_migrations_table().await?;
    let applied = conn2.list_applied_migrations().await?;
    let count = applied.len();

    if count > 0 {
        println!("Rolling back {count} migration(s)...");
        let migrator = Migrator::new(Path::new("./database/migrations")).await?;
        migrator.undo(pool, 0).await?;
    }

    println!("Running all migrations...");
    run(pool).await?;
    println!("Refresh complete");
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
