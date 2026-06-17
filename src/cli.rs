use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Start the HTTP server (runs migrations automatically)
    Server,
    /// Database migration commands
    Db {
        #[command(subcommand)]
        command: DbCommand,
    },
}

#[derive(clap::Subcommand)]
pub enum DbCommand {
    /// Apply all pending migrations
    Migrate,
    /// Roll back the last applied migration
    Rollback,
    /// Drop all tables and re-run all migrations
    Fresh,
    /// Roll back all migrations, then re-apply them
    Refresh,
    /// Show migration status (applied vs pending)
    Status,
}
