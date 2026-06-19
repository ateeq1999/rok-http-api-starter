pub mod auth;
pub mod login_otp;
pub mod magic_link;
pub mod oauth;
pub mod otp;
pub mod rbac;
pub mod sessions;
pub mod two_factor;

use axum::response::{IntoResponse, Response};

use crate::context::AuthContext;
use crate::middleware::AuthStrategy;
use crate::primitives::TokenPair;

pub(crate) fn token_response<C: AuthContext>(ctx: &C, tokens: &TokenPair, message: &str) -> Response {
    use api_core::response::ApiResponse;
    use axum::http::header;

    if ctx.config().auth_strategy == AuthStrategy::Cookie {
        let access_cookie = format!(
            "access_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
            tokens.access_token,
            ctx.config().token_ttl.as_secs(),
        );
        let refresh_cookie = format!(
            "refresh_token={}; Path=/api/v1/auth/refresh; HttpOnly; SameSite=Lax; Max-Age={}",
            tokens.refresh_token,
            ctx.config().refresh_ttl.as_secs(),
        );
        let body = ApiResponse::ok(serde_json::json!({ "message": message }));
        let mut response = body.into_response();
        let headers = response.headers_mut();
        headers.append(header::SET_COOKIE, access_cookie.parse().unwrap());
        headers.append(header::SET_COOKIE, refresh_cookie.parse().unwrap());
        response
    } else {
        ApiResponse::ok(serde_json::json!(tokens)).into_response()
    }
}
