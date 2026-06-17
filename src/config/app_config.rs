use std::time::Duration;

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub auth_secret: String,
    pub token_ttl: Duration,
    pub refresh_ttl: Duration,
    pub smtp_host: String,
    pub smtp_port: u16,
    #[allow(dead_code)]
    pub smtp_username: String,
    #[allow(dead_code)]
    pub smtp_password: String,
    pub smtp_from: String,
    #[allow(dead_code)]
    pub google_client_id: String,
    #[allow(dead_code)]
    pub google_client_secret: String,
    #[allow(dead_code)]
    pub google_redirect_uri: String,
    pub app_url: String,
    pub otp_length: u32,
    pub storage_dir: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/axum_app".into()),
            auth_secret: std::env::var("AUTH_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".into()),
            token_ttl: Duration::from_secs(
                std::env::var("TOKEN_TTL")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3600),
            ),
            refresh_ttl: Duration::from_secs(
                std::env::var("REFRESH_TTL")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(86400 * 30),
            ),
            smtp_host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".into()),
            smtp_port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1025),
            smtp_username: std::env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: std::env::var("SMTP_PASSWORD").unwrap_or_default(),
            smtp_from: std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@axum-app.dev".into()),
            google_client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            google_redirect_uri: std::env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:8080/auth/google/callback".into()),
            app_url: std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:8080".into()),
            otp_length: std::env::var("OTP_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6)
                .clamp(4, 8),
            storage_dir: std::env::var("STORAGE_DIR")
                .unwrap_or_else(|_| "./storage".into()),
        }
    }
}
