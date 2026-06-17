use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
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

impl AuthError {
    pub fn internal(msg: impl ToString) -> Self { Self::Internal(msg.to_string()) }
    pub fn bad_request(msg: impl ToString) -> Self { Self::BadRequest(msg.to_string()) }
    pub fn unauthorized(msg: impl ToString) -> Self { Self::Unauthorized(msg.to_string()) }
    pub fn not_found(msg: impl ToString) -> Self { Self::NotFound(msg.to_string()) }
    pub fn forbidden(msg: impl ToString) -> Self { Self::Forbidden(msg.to_string()) }
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::NotFound("resource not found".into()),
            other => Self::Database(other.to_string()),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (code, msg) = match &self {
            Self::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            Self::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            Self::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            Self::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            Self::Database(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
            Self::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        let body = serde_json::json!({
            "error": {
                "code": code.canonical_reason().unwrap_or("ERROR"),
                "message": msg,
            }
        });
        (code, Json(body)).into_response()
    }
}
