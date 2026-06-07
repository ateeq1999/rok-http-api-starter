use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Start the HTTP server
    Server {
        /// Run pending migrations before starting
        #[arg(long)]
        run_migrations: bool,
    },
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
    /// Roll back the last batch of migrations
    Rollback,
    /// Show migration status (applied vs pending)
    Status,
}
