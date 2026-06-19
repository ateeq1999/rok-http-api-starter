use axum::routing::{get, post};
use axum::Router;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

use crate::context::AuthContext;
use crate::handlers;

// ─── Plugin Builder ───────────────────────────────────────

pub struct AuthPlugin {
    pub(crate) magic_link: bool,
    pub(crate) login_otp: bool,
    pub(crate) totp_2fa: bool,
    pub(crate) sessions: bool,
    pub(crate) google: bool,
    pub(crate) github: bool,
}

impl AuthPlugin {
    pub fn builder() -> AuthPluginBuilder {
        AuthPluginBuilder {
            magic_link: false,
            login_otp: false,
            totp_2fa: false,
            sessions: false,
            google: false,
            github: false,
        }
    }

    pub fn public_routes<C: AuthContext>(&self) -> Router<C> {
        let mut routes = Router::new()
            .route("/register", post(handlers::auth::register::<C>))
            .route("/login", post(handlers::auth::login::<C>))
            .route("/refresh", post(handlers::auth::refresh::<C>))
            .route("/forgot-password", post(handlers::auth::forgot_password::<C>))
            .route("/reset-password", post(handlers::auth::reset_password::<C>));

        if self.magic_link {
            routes = routes
                .route("/magic-link", post(handlers::magic_link::request::<C>))
                .route("/magic-link/verify", get(handlers::magic_link::verify::<C>));
        }

        if self.login_otp {
            routes = routes
                .route("/otp/login/send", post(handlers::login_otp::send::<C>))
                .route("/otp/login/verify", post(handlers::login_otp::verify::<C>));
        }

        if self.google {
            routes = routes
                .route("/oauth/google/redirect", get(handlers::oauth::redirect::<C>))
                .route("/oauth/google/callback", get(handlers::oauth::callback::<C>));
        }

        if self.github {
            routes = routes
                .route("/oauth/github/redirect", get(handlers::oauth::redirect::<C>))
                .route("/oauth/github/callback", get(handlers::oauth::callback::<C>));
        }

        // Apply rate limiting to strict routes
        let strict_config = GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(5)
            .finish()
            .unwrap();

        let strict_routes = Router::new()
            .route("/register", post(handlers::auth::register::<C>))
            .route("/login", post(handlers::auth::login::<C>))
            .layer(GovernorLayer::new(strict_config));

        routes.merge(strict_routes)
    }

    pub fn protected_routes<C: AuthContext>(&self) -> Router<C> {
        let mut routes = Router::new()
            .route("/logout", post(handlers::auth::logout::<C>));

        if self.totp_2fa {
            routes = routes
                .route("/2fa/enable", post(handlers::two_factor::enable::<C>))
                .route("/2fa/verify", post(handlers::two_factor::verify::<C>))
                .route("/2fa/disable", post(handlers::two_factor::disable::<C>));
        }

        if self.sessions {
            routes = routes
                .route("/me/sessions", get(handlers::sessions::list::<C>))
                .route("/me/sessions/{id}", axum::routing::delete(handlers::sessions::revoke::<C>))
                .route("/me/sessions", axum::routing::delete(handlers::sessions::revoke_all::<C>));
        }

        routes = routes
            .route("/otp/send", post(handlers::otp::send::<C>))
            .route("/otp/verify", post(handlers::otp::verify::<C>));

        routes
    }
}

pub struct AuthPluginBuilder {
    magic_link: bool,
    login_otp: bool,
    totp_2fa: bool,
    sessions: bool,
    google: bool,
    github: bool,
}

impl AuthPluginBuilder {
    pub fn magic_link(mut self) -> Self { self.magic_link = true; self }
    pub fn login_otp(mut self) -> Self { self.login_otp = true; self }
    pub fn totp_2fa(mut self) -> Self { self.totp_2fa = true; self }
    pub fn sessions(mut self) -> Self { self.sessions = true; self }
    pub fn google(mut self) -> Self { self.google = true; self }
    pub fn github(mut self) -> Self { self.github = true; self }

    pub fn build(self) -> AuthPlugin {
        AuthPlugin {
            magic_link: self.magic_link,
            login_otp: self.login_otp,
            totp_2fa: self.totp_2fa,
            sessions: self.sessions,
            google: self.google,
            github: self.github,
        }
    }
}
