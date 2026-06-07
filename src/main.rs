mod auth;
mod cli;
mod config;
mod controllers;
mod db;
mod error;
mod guards;
mod mail;
mod migrations;
mod models;
mod response;
mod routes;
mod services;
mod social;
mod state;
mod validators;

use clap::Parser;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cli::{Cli, Command, DbCommand};
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Command::Server { run_migrations: false }) => {
            start_server().await?;
        }
        Some(Command::Server { run_migrations: true }) => {
            let config = config::AppConfig::from_env();
            let pool = PgPool::connect(&config.database_url).await?;
            migrations::run(&pool).await?;
            db::init(pool.clone());
            serve(config, pool).await?;
        }
        Some(Command::Db { command }) => {
            let config = config::AppConfig::from_env();
            let pool = PgPool::connect(&config.database_url).await?;
            match command {
                DbCommand::Migrate => {
                    migrations::run(&pool).await?;
                    println!("Migrations complete");
                }
                DbCommand::Rollback => {
                    migrations::rollback(&pool).await?;
                }
                DbCommand::Fresh => {
                    migrations::fresh(&pool).await?;
                }
                DbCommand::Refresh => {
                    migrations::refresh(&pool).await?;
                }
                DbCommand::Status => {
                    migrations::status(&pool).await?;
                }
            }
        }
    }

    Ok(())
}

async fn start_server() -> anyhow::Result<()> {
    let config = config::AppConfig::from_env();
    let pool = PgPool::connect(&config.database_url).await?;
    db::init(pool.clone());
    serve(config, pool).await
}

async fn serve(config: config::AppConfig, pool: PgPool) -> anyhow::Result<()> {
    let mailer = mail::Mailer::new(
        &config.smtp_host,
        config.smtp_port,
        &config.smtp_from,
    )?;

    let app_state = AppState {
        pool,
        config,
        mailer,
    };

    let app = routes::app_router()
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("server listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await?;

    Ok(())
}
