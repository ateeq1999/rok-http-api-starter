mod auth;
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

use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = config::AppConfig::from_env();
    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("failed to connect to database");

    migrations::run(&pool)
        .await
        .expect("failed to run migrations");

    db::init(pool.clone());

    let mailer = mail::Mailer::new(
        &config.smtp_host,
        config.smtp_port,
        &config.smtp_from,
    )
    .expect("failed to create mailer");

    let app_state = AppState {
        pool,
        config,
        mailer,
    };

    let app = routes::app_router()
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("failed to bind");

    tracing::info!("server listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await.expect("server error");
}
