use std::sync::OnceLock;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use rok_api_start::routes;

static POOL: OnceLock<PgPool> = OnceLock::new();

async fn db_pool() -> &'static PgPool {
    POOL.get_or_init(|| {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/axum_app_test".into());
        PgPool::connect_lazy(&url).expect("failed to create test pool")
    });
    POOL.get().unwrap()
}

async fn app() -> Router {
    let pool = db_pool().await;
    api_core::db::init(pool.clone());
    let config = rok_api_start::config::AppConfig::from_env();
    let mailer = rok_api_start::mail::Mailer::new(
        &config.smtp_host,
        config.smtp_port,
        &config.smtp_from,
    )
    .unwrap();
    let state = rok_api_start::state::AppState { pool: pool.clone(), config, mailer };
    routes::app_router().with_state(state)
}

#[tokio::test]
async fn health_returns_200() {
    let app = app().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|b| serde_json::from_slice(&b).unwrap())
        .unwrap();
    assert_eq!(body["data"]["status"], "ok");
}

#[tokio::test]
async fn health_returns_json_content_type() {
    let app = app().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.headers()["content-type"],
        "application/json"
    );
}
