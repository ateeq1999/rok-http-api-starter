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
