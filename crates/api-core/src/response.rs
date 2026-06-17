use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;

pub struct ApiResponse {
    status: StatusCode,
    body: Value,
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl ApiResponse {
    pub fn ok(data: Value) -> Self {
        Self { status: StatusCode::OK, body: serde_json::json!({ "data": data }) }
    }

    pub fn created(data: Value) -> Self {
        Self { status: StatusCode::CREATED, body: serde_json::json!({ "data": data }) }
    }

    pub fn no_content() -> Self {
        Self { status: StatusCode::NO_CONTENT, body: serde_json::json!(null) }
    }

    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        let body = serde_json::json!({
            "error": {
                "code": code.code_str(),
                "message": message.into(),
            }
        });
        Self { status: code.status(), body }
    }

    pub fn paginated(data: Value, total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Self {
            status: StatusCode::OK,
            body: serde_json::json!({
                "data": data,
                "meta": {
                    "total": total,
                    "page": page,
                    "per_page": per_page,
                    "total_pages": total_pages,
                }
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
    TooManyRequests,
    InternalServerError,
}

impl ErrorCode {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn code_str(&self) -> &'static str {
        match self {
            Self::BadRequest => "E_BAD_REQUEST",
            Self::Unauthorized => "E_UNAUTHORIZED",
            Self::Forbidden => "E_FORBIDDEN",
            Self::NotFound => "E_NOT_FOUND",
            Self::Conflict => "E_CONFLICT",
            Self::UnprocessableEntity => "E_VALIDATION",
            Self::TooManyRequests => "E_TOO_MANY_REQUESTS",
            Self::InternalServerError => "E_INTERNAL_SERVER_ERROR",
        }
    }
}
