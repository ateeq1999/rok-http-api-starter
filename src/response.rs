use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;

#[derive(serde::Serialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
}

pub fn ok(data: Value) -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(data))
}

pub fn created(data: Value) -> (StatusCode, Json<Value>) {
    (StatusCode::CREATED, Json(data))
}

pub fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}

pub fn error(code: &str, message: &str, status: u16) -> (StatusCode, Json<Value>) {
    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = serde_json::json!({
        "error": code,
        "message": message,
    });
    (status_code, Json(body))
}
