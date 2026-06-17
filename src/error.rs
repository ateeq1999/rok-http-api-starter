use axum::response::{IntoResponse, Response};

use api_core::response::{ApiResponse, ErrorCode};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
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

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (code, msg) = match &self {
            Self::NotFound(m) => (ErrorCode::NotFound, m.clone()),
            Self::Unauthorized(m) => (ErrorCode::Unauthorized, m.clone()),
            Self::Forbidden(m) => (ErrorCode::Forbidden, m.clone()),
            Self::BadRequest(m) => (ErrorCode::BadRequest, m.clone()),
            Self::Database(m) => (ErrorCode::InternalServerError, m.clone()),
            Self::Internal(m) => (ErrorCode::InternalServerError, m.clone()),
        };
        ApiResponse::error(code, msg).into_response()
    }
}

/// Extension trait to reduce `.map_err` verbosity on Results.
pub trait OrInternal<T> {
    fn or_internal(self) -> Result<T, AppError>;
    fn or_not_found(self) -> Result<T, AppError>;
}

impl<T> OrInternal<T> for Result<T, sqlx::Error> {
    fn or_internal(self) -> Result<T, AppError> {
        self.map_err(AppError::from)
    }

    fn or_not_found(self) -> Result<T, AppError> {
        self.map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound("not found".into()),
            other => AppError::Database(other.to_string()),
        })
    }
}
