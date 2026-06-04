mod config;
mod controllers;
mod error;
mod guards;
mod migrations;
mod models;
mod routes;
mod social;
mod state;
mod validators;

use std::sync::Arc;

use rok_auth::Auth;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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

    let auth = Arc::new(Auth::new(config.auth_config()).expect("Auth secret must not be empty"));
    let app_state = state::AppState {
        pool: pool.clone(),
        auth: auth.clone(),
        config: config,
    };

    let app = routes::app_router()
        .layer(rok_auth::axum::AuthLayer::new((*auth).clone()))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("failed to bind");

    tracing::info!("server listening on http://127.0.0.1:8080");
    axum::serve(listener, app).await.expect("server error");
}
