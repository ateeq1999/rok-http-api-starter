use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::PgPool;

use di::{module, Container, ContainerBuilder, Module};

use crate::app::mails::Mailer;
use crate::app::repositories::{
    AppPermissionFinder, AppUserFinder, AvatarStorage, LocalAvatarStorage, PgUserRepository,
    UserRepository,
};
use crate::app::services::user_service::UserService;
use crate::config::AppConfig;

/// The root DI module: every provider the app needs, wired leaf-first. `crates/auth`'s own
/// services/handlers are *not* migrated onto this container — they keep using their existing
/// `AuthContext` pattern (see the `AuthContext` impl below), which is bridged in via
/// `AppUserFinder`/`AppPermissionFinder`/`Mailer` rather than rewritten.
#[module(
    providers = [
        PgUserRepository as dyn UserRepository,
        LocalAvatarStorage as dyn AvatarStorage,
        UserService,
        AppUserFinder as dyn auth::context::UserFinder,
        AppPermissionFinder as dyn auth::context::PermissionFinder,
    ],
)]
pub struct AppModule;

/// Axum shared state. `container` is how new code resolves providers (via `di::Injected<T>`);
/// the remaining fields are cached once at bootstrap, from the same container, purely to satisfy
/// `auth::context::AuthContext`'s borrow-shaped accessors (`fn mailer(&self) -> &dyn MailSender`)
/// which can't be served by a container lookup that returns an owned `Arc` each time.
#[derive(Clone)]
pub struct AppState {
    pub container: Container,
    pool: Arc<PgPool>,
    auth_config: Arc<auth::config::AuthConfig>,
    mailer: Arc<dyn auth::context::MailSender>,
    user_finder: Arc<dyn auth::context::UserFinder>,
    permission_finder: Arc<dyn auth::context::PermissionFinder>,
}

impl FromRef<AppState> for Container {
    fn from_ref(state: &AppState) -> Self {
        state.container.clone()
    }
}

impl auth::context::AuthContext for AppState {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
    fn config(&self) -> &auth::config::AuthConfig {
        &self.auth_config
    }
    fn mailer(&self) -> &dyn auth::context::MailSender {
        self.mailer.as_ref()
    }
    fn user_finder(&self) -> &dyn auth::context::UserFinder {
        self.user_finder.as_ref()
    }
    fn permission_finder(&self) -> &dyn auth::context::PermissionFinder {
        self.permission_finder.as_ref()
    }
}

/// The composition root: the one place fallible/async construction (`PgPool::connect`,
/// `Mailer::new`'s SMTP transport) meets the DI container. `Injectable::construct` is always
/// synchronous, so both of those are built by the caller and handed in as pre-built leaves.
pub fn bootstrap(config: AppConfig, pool: PgPool, mailer: Mailer) -> anyhow::Result<AppState> {
    let auth_config = Arc::new(build_auth_config(&config));

    let mut builder = ContainerBuilder::new();
    builder.insert(pool);
    builder.insert(config);
    builder.insert_arc(auth_config.clone());
    builder
        .bind::<dyn auth::context::MailSender>()
        .to_arc(Arc::new(mailer) as Arc<dyn auth::context::MailSender>);

    AppModule::register(&mut builder)?;
    let container = builder.build();

    let pool = container
        .get::<PgPool>()
        .expect("PgPool was inserted before AppModule::register");
    let mailer = container
        .get::<dyn auth::context::MailSender>()
        .expect("MailSender was bound before AppModule::register");
    let user_finder = container
        .get::<dyn auth::context::UserFinder>()
        .expect("AppUserFinder is registered by AppModule");
    let permission_finder = container
        .get::<dyn auth::context::PermissionFinder>()
        .expect("AppPermissionFinder is registered by AppModule");

    Ok(AppState {
        container,
        pool,
        auth_config,
        mailer,
        user_finder,
        permission_finder,
    })
}

fn build_auth_config(config: &AppConfig) -> auth::config::AuthConfig {
    auth::config::AuthConfig {
        auth_secret: config.auth_secret.clone(),
        auth_strategy: config.auth_strategy.clone(),
        token_ttl: config.token_ttl,
        refresh_ttl: config.refresh_ttl,
        app_url: config.app_url.clone(),
        otp_length: config.otp_length,
        google: if config.google_client_id.is_empty() {
            None
        } else {
            Some(auth::config::OAuthProviderConfig {
                client_id: config.google_client_id.clone(),
                client_secret: config.google_client_secret.clone(),
                redirect_uri: config.google_redirect_uri.clone(),
            })
        },
        github: if config.github_client_id.is_empty() {
            None
        } else {
            Some(auth::config::OAuthProviderConfig {
                client_id: config.github_client_id.clone(),
                client_secret: config.github_client_secret.clone(),
                redirect_uri: config.github_redirect_uri.clone(),
            })
        },
    }
}
