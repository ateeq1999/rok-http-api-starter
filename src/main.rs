use clap::Parser;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use axum::http::header;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use rok_api_start::cli::{Cli, Command, DbCommand};
use rok_api_start::config::AppConfig;
use api_core::db;
use rok_api_start::app::mails::Mailer;
use rok_api_start::start::routes;
use rok_api_start::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let _ = dotenvy::dotenv();
    let cli = Cli::parse();

    match cli.command {
        None | Some(Command::Server) => {
            let config = AppConfig::from_env();
            let pool = PgPool::connect(&config.database_url).await?;
            api_core::migrations::run(&pool).await?;
            db::init(pool.clone());
            serve(config, pool).await?;
        }
        Some(Command::Db { command }) => {
            let config = AppConfig::from_env();
            let pool = PgPool::connect(&config.database_url).await?;
            match command {
                DbCommand::Migrate => {
                    api_core::migrations::run(&pool).await?;
                    println!("Migrations complete");
                }
                DbCommand::Rollback => {
                    api_core::migrations::rollback(&pool).await?;
                }
                DbCommand::Fresh => {
                    api_core::migrations::fresh(&pool).await?;
                }
                DbCommand::Refresh => {
                    api_core::migrations::refresh(&pool).await?;
                }
                DbCommand::Status => {
                    api_core::migrations::status(&pool).await?;
                }
            }
        }
    }

    Ok(())
}

async fn serve(config: AppConfig, pool: PgPool) -> anyhow::Result<()> {
    let mailer = Mailer::new(
        &config.smtp_host,
        config.smtp_port,
        &config.smtp_from,
    )?;

    let app_state = AppState {
        pool,
        config,
        mailer,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let security_headers = SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );

    let frame_options = SetResponseHeaderLayer::overriding(
        header::X_FRAME_OPTIONS,
        header::HeaderValue::from_static("DENY"),
    );

    let hsts = SetResponseHeaderLayer::overriding(
        header::HeaderName::from_static("strict-transport-security"),
        header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    let app = routes::app_router(&app_state.config.auth_secret)
        .layer(cors)
        .layer(security_headers)
        .layer(frame_options)
        .layer(hsts)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("server listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await?;

    Ok(())
}
