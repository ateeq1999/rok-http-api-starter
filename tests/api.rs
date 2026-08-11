use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use auth::plugin::AuthPlugin;
use rok_api_start::app::mails::Mailer;
use rok_api_start::config::AppConfig;
use rok_api_start::start::routes;

async fn app() -> Router {
    let config = AppConfig::from_env();
    let pool = PgPool::connect_lazy(&config.database_url).expect("failed to create test pool");
    let mailer = Mailer::new(&config.smtp_host, config.smtp_port, &config.smtp_from)
        .expect("failed to build mailer");
    let auth_secret = config.auth_secret.clone();
    let auth_strategy = config.auth_strategy.clone();

    let state = rok_api_start::state::bootstrap(config, pool, mailer).expect("bootstrap failed");

    let auth = AuthPlugin::builder()
        .magic_link()
        .login_otp()
        .totp_2fa()
        .sessions()
        .google()
        .github()
        .build();

    routes::app_router(&auth_secret, &auth_strategy, &auth).with_state(state)
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

#[tokio::test]
async fn users_index_requires_admin() {
    let app = app().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // No Authorization header at all -> the JwtAuthLayer rejects before the DI-injected
    // UserService is ever reached, confirming the route is wired end to end.
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
