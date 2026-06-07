use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::response::ErrorBody;

#[derive(Debug)]
pub enum AppError {
  Database(String),
  NotFound(String),
  #[allow(dead_code)]
  Forbidden(String),
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
    let (status, code, msg) = match &self {
      Self::NotFound(m) => (StatusCode::NOT_FOUND, "E_ROW_NOT_FOUND", m),
      Self::Forbidden(m) => (StatusCode::FORBIDDEN, "E_FORBIDDEN", m),
      Self::BadRequest(m) => (StatusCode::BAD_REQUEST, "E_BAD_REQUEST", m),
      Self::Database(m) => (StatusCode::INTERNAL_SERVER_ERROR, "E_DATABASE", m),
      Self::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, "E_INTERNAL", m),
    };
    (
      status,
      Json(ErrorBody {
        error: code.to_string(),
        message: msg.clone(),
      }),
    )
      .into_response()
  }
}

impl std::fmt::Display for AppError {
  fn fmt(
    &self,
    f: &mut std::fmt::Formatter<'_>,
  ) -> std::fmt::Result {
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
