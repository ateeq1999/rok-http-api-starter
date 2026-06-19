use axum::extract::FromRef;

use crate::config::AppConfig;
use crate::app::mails::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub config: AppConfig,
    pub mailer: Mailer,
}

impl FromRef<AppState> for AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for sqlx::PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Mailer {
    fn from_ref(state: &AppState) -> Self {
        state.mailer.clone()
    }
}

// ── Auth crate trait implementations ──────────────────────

#[async_trait::async_trait]
impl auth::context::MailSender for Mailer {
    async fn send_password_reset(&self, to: &str, name: &str, token: &str, url: &str) -> Result<(), String> {
        self.send_password_reset(to, name, token, url).await.map_err(|e| e.to_string())
    }
    async fn send_magic_link(&self, to: &str, name: &str, url: &str) -> Result<(), String> {
        self.send_magic_link(to, name, url).await.map_err(|e| e.to_string())
    }
    async fn send_login_otp(&self, to: &str, name: &str, code: &str, url: &str) -> Result<(), String> {
        self.send_login_otp(to, name, code, url).await.map_err(|e| e.to_string())
    }
    async fn send_otp(&self, to: &str, name: &str, code: &str, url: &str) -> Result<(), String> {
        self.send_otp(to, name, code, url).await.map_err(|e| e.to_string())
    }
}

#[async_trait::async_trait]
impl auth::context::UserFinder for AppState {
    async fn find_by_email(&self, email: &str) -> Result<Option<auth::context::UserRecord>, sqlx::Error> {
        use crate::app::models::User;
        Ok(User::find_by_email(email).await?.map(auth::context::UserRecord::from))
    }
    async fn find_by_identifier(&self, identifier: &str) -> Result<Option<auth::context::UserRecord>, sqlx::Error> {
        use crate::app::models::User;
        Ok(User::find_by_identifier(identifier).await?.map(auth::context::UserRecord::from))
    }
    async fn find_or_fail(&self, id: &str) -> Result<auth::context::UserRecord, sqlx::Error> {
        use crate::app::models::User;
        use api_core::crud::CrudService;
        User::find_or_fail(id).await.map(auth::context::UserRecord::from)
    }
    async fn create_user(&self, fields: &[(&str, &str)]) -> Result<auth::context::UserRecord, sqlx::Error> {
        use crate::app::models::User;
        use api_core::crud::{CrudService, FieldValue};
        let fv_fields: Vec<(&str, FieldValue)> = fields.iter()
            .map(|(k, v)| (*k, FieldValue::String(v.to_string())))
            .collect();
        User::create(&fv_fields).await.map(auth::context::UserRecord::from)
    }
    async fn update_user(&self, id: &str, fields: &[(&str, Option<&str>)]) -> Result<auth::context::UserRecord, sqlx::Error> {
        use crate::app::models::User;
        use api_core::crud::{CrudService, FieldValue};
        let fv_fields: Vec<(&str, FieldValue)> = fields.iter()
            .map(|(k, v)| (*k, match v {
                Some(val) => FieldValue::String(val.to_string()),
                None => FieldValue::OptionString(None),
            }))
            .collect();
        User::update(id, &fv_fields).await.map(auth::context::UserRecord::from)
    }
}

impl From<crate::app::models::User> for auth::context::UserRecord {
    fn from(u: crate::app::models::User) -> Self {
        auth::context::UserRecord {
            id: u.id,
            email: u.email,
            password_hash: u.password_hash,
            name: u.name,
            roles: u.roles,
            username: u.username,
            avatar_url: u.avatar_url,
            email_verified_at: u.email_verified_at,
            totp_secret: u.totp_secret,
            totp_enabled: u.totp_enabled,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

impl auth::context::AuthContext for AppState {
    fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
    fn config(&self) -> &auth::context::AuthConfig {
        // Leak the config to get a 'static reference
        // In production, use Arc or once_cell
        Box::leak(Box::new(auth::context::AuthConfig {
            auth_secret: self.config.auth_secret.clone(),
            auth_strategy: self.config.auth_strategy.clone(),
            token_ttl: self.config.token_ttl,
            refresh_ttl: self.config.refresh_ttl,
            app_url: self.config.app_url.clone(),
            otp_length: self.config.otp_length,
            google: if self.config.google_client_id.is_empty() {
                None
            } else {
                Some(auth::context::OAuthProviderConfig {
                    client_id: self.config.google_client_id.clone(),
                    client_secret: self.config.google_client_secret.clone(),
                    redirect_uri: self.config.google_redirect_uri.clone(),
                })
            },
            github: if self.config.github_client_id.is_empty() {
                None
            } else {
                Some(auth::context::OAuthProviderConfig {
                    client_id: self.config.github_client_id.clone(),
                    client_secret: self.config.github_client_secret.clone(),
                    redirect_uri: self.config.github_redirect_uri.clone(),
                })
            },
        }))
    }
    fn mailer(&self) -> &dyn auth::context::MailSender {
        &self.mailer
    }
    fn user_finder(&self) -> &dyn auth::context::UserFinder {
        self
    }
    fn permission_finder(&self) -> &dyn auth::context::PermissionFinder {
        self
    }
}

#[async_trait::async_trait]
impl auth::context::PermissionFinder for AppState {
    async fn get_user_permissions(&self, user_id: &str) -> Result<String, sqlx::Error> {
        let perms: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT p.name FROM user_roles ur
             JOIN role_permissions rp ON rp.role_id = ur.role_id
             JOIN permissions p ON p.id = rp.permission_id
             WHERE ur.user_id = $1
             ORDER BY p.name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(perms.join(","))
    }
}

impl From<auth::error::AuthError> for crate::error::AppError {
    fn from(e: auth::error::AuthError) -> Self {
        match e {
            auth::error::AuthError::Database(m) => Self::Database(m),
            auth::error::AuthError::NotFound(m) => Self::NotFound(m),
            auth::error::AuthError::Unauthorized(m) => Self::Unauthorized(m),
            auth::error::AuthError::Forbidden(m) => Self::Forbidden(m),
            auth::error::AuthError::BadRequest(m) => Self::BadRequest(m),
            auth::error::AuthError::Internal(m) => Self::Internal(m),
        }
    }
}
