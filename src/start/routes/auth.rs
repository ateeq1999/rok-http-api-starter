use axum::routing::post;
use axum::Router;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

use crate::app::controllers::auth_controller;
use crate::state::AppState;

/// Public auth routes (no JWT required).
pub fn public_routes() -> Router<AppState> {
    // Strict rate limit: burst of 5, replenish 1/sec
    let strict_config = GovernorConfigBuilder::default()
        .per_second(1)
        .burst_size(5)
        .finish()
        .unwrap();

    // Generous rate limit: burst of 10, replenish 2/sec
    let generous_config = GovernorConfigBuilder::default()
        .per_second(2)
        .burst_size(10)
        .finish()
        .unwrap();

    let strict_routes = Router::new()
        .route("/register", post(auth_controller::register))
        .route("/login", post(auth_controller::login))
        .route("/otp/login/send", post(auth_controller::login_otp_send))
        .route("/otp/login/verify", post(auth_controller::login_otp_verify))
        .layer(GovernorLayer::new(strict_config));

    let generous_routes = Router::new()
        .route(
            "/forgot-password",
            post(auth_controller::forgot_password),
        )
        .route(
            "/magic-link",
            post(auth_controller::magic_link_request),
        )
        .layer(GovernorLayer::new(generous_config));

    let unthrottled_routes = Router::new()
        .route("/reset-password", post(auth_controller::reset_password))
        .route("/magic-link/verify", post(auth_controller::magic_link_verify));

    strict_routes.merge(generous_routes).merge(unthrottled_routes)
}

/// Protected auth routes (JWT required).
pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/logout", post(auth_controller::logout))
        .route("/refresh", post(auth_controller::refresh))
}
