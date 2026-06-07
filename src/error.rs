use axum::response::{IntoResponse, Response};

use crate::response::{ApiResponse, ErrorCode};

#[derive(Debug)]
pub enum AppError {
    Database(String),
    NotFound(String),
    #[allow(dead_code)]
    Forbidden(String),
    #[allow(dead_code)]
    BadRequest(String),
    #[allow(dead_code)]
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
        let (code, msg) = match self {
            Self::NotFound(m) => (ErrorCode::NotFound, m),
            Self::Forbidden(m) => (ErrorCode::Forbidden, m),
            Self::BadRequest(m) => (ErrorCode::BadRequest, m),
            Self::Database(m) => (ErrorCode::InternalServerError, m),
            Self::Internal(m) => (ErrorCode::InternalServerError, m),
        };
        ApiResponse::error(code, msg).into_response()
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
