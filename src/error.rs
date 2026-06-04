use axum::response::{IntoResponse, Response};
use rok_core::api::ApiResponse;

/// Application-level error that converts to `ApiResponse` JSON.
/// Handlers returning `Result<ApiResponse, AppError>` can use `?`.
#[derive(Debug)]
pub enum AppError {
    Database(String),
    NotFound(String),
    Forbidden(String),
    #[allow(dead_code)]
    BadRequest(String),
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::NotFound("resource not found".into()),
            other => Self::Database(other.to_string()),
        }
    }
}

impl From<rok_auth::AuthError> for AppError {
    fn from(e: rok_auth::AuthError) -> Self {
        match e {
            rok_auth::AuthError::InvalidToken => Self::Forbidden("invalid or expired token".into()),
            rok_auth::AuthError::Forbidden(msg) => Self::Forbidden(msg),
            _ => Self::Internal(e.to_string()),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (code, status) = match &self {
            Self::NotFound(_) => ("E_ROW_NOT_FOUND", 404),
            Self::Forbidden(_) => ("E_FORBIDDEN", 403),
            Self::BadRequest(_) => ("E_BAD_REQUEST", 400),
            Self::Database(_) => ("E_DATABASE", 500),
            Self::Internal(_) => ("E_INTERNAL", 500),
        };
        ApiResponse::error(code, self.to_string(), status).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(msg) => write!(f, "{msg}"),
            Self::NotFound(msg) => write!(f, "{msg}"),
            Self::Forbidden(msg) => write!(f, "{msg}"),
            Self::BadRequest(msg) => write!(f, "{msg}"),
            Self::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for AppError {}
