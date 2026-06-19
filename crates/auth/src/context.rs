use sqlx::PgPool;

use crate::middleware::AuthStrategy;

#[derive(Clone)]
pub struct OAuthProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Clone)]
pub struct AuthConfig {
    pub auth_secret: String,
    pub auth_strategy: AuthStrategy,
    pub token_ttl: std::time::Duration,
    pub refresh_ttl: std::time::Duration,
    pub app_url: String,
    pub otp_length: u32,
    pub google: Option<OAuthProviderConfig>,
    pub github: Option<OAuthProviderConfig>,
}

#[async_trait::async_trait]
pub trait MailSender: Send + Sync {
    async fn send_password_reset(&self, to: &str, name: &str, token: &str, url: &str) -> Result<(), String>;
    async fn send_magic_link(&self, to: &str, name: &str, url: &str) -> Result<(), String>;
    async fn send_login_otp(&self, to: &str, name: &str, code: &str, url: &str) -> Result<(), String>;
    async fn send_otp(&self, to: &str, name: &str, code: &str, url: &str) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub roles: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait::async_trait]
pub trait UserFinder: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRecord>, sqlx::Error>;
    async fn find_by_identifier(&self, identifier: &str) -> Result<Option<UserRecord>, sqlx::Error>;
    async fn find_or_fail(&self, id: &str) -> Result<UserRecord, sqlx::Error>;
    async fn create_user(&self, fields: &[(&str, &str)]) -> Result<UserRecord, sqlx::Error>;
    async fn update_user(&self, id: &str, fields: &[(&str, Option<&str>)]) -> Result<UserRecord, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait PermissionFinder: Send + Sync {
    async fn get_user_permissions(&self, user_id: &str) -> Result<String, sqlx::Error>;
}

pub trait AuthContext: Clone + Send + Sync + 'static {
    fn pool(&self) -> &PgPool;
    fn config(&self) -> &AuthConfig;
    fn mailer(&self) -> &dyn MailSender;
    fn user_finder(&self) -> &dyn UserFinder;
    fn permission_finder(&self) -> &dyn PermissionFinder;
}
