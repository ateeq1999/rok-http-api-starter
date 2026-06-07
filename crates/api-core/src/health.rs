use crate::response::ApiResponse;

pub async fn get() -> ApiResponse {
    ApiResponse::ok(serde_json::json!({ "status": "ok" }))
}
