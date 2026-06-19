use api_core::response::ApiResponse;
use axum::extract::State;
use axum::response::Response;

use crate::context::AuthContext;
use crate::error::AuthError;
use super::token_response;
use crate::services;
use crate::validators::ValidatedJson;
use crate::validators::{MagicLinkRequest};

pub async fn request<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<MagicLinkRequest>,
) -> Result<ApiResponse, AuthError> {
    services::magic_link_service::request_magic_link(&ctx, &body.email).await?;
    Ok(ApiResponse::message("magic link sent"))
}

pub async fn verify<C: AuthContext>(
    State(ctx): State<C>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Response, AuthError> {
    let token = params.get("token")
        .ok_or_else(|| AuthError::bad_request("missing token parameter"))?;
    let tokens = services::magic_link_service::verify_magic_link(&ctx, token).await?;
    Ok(token_response(&ctx, &tokens, "authenticated via magic link"))
}
