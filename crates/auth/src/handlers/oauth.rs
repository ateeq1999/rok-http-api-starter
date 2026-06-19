use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;

use crate::context::AuthContext;
use crate::error::AuthError;
use super::token_response;
use crate::services;

pub async fn redirect<C: AuthContext>(
    State(ctx): State<C>,
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<Response, AuthError> {
    let config = ctx.config();
    let provider_config = match provider.as_str() {
        "google" => config.google.as_ref().ok_or_else(|| AuthError::internal("Google OAuth not configured"))?,
        "github" => config.github.as_ref().ok_or_else(|| AuthError::internal("GitHub OAuth not configured"))?,
        _ => return Err(AuthError::bad_request(format!("unsupported provider: {provider}"))),
    };

    let (auth_url, state, pkce_verifier) = services::oauth_service::start_authorization(
        &provider,
        provider_config,
        &config.app_url,
    )?;

    let state_cookie = format!(
        "oauth_state={state}; Path=/; HttpOnly; SameSite=Lax; Max-Age=300"
    );
    let pkce_cookie = format!(
        "oauth_pkce={pkce_verifier}; Path=/; HttpOnly; SameSite=Lax; Max-Age=300"
    );

    let mut response = axum::response::Redirect::temporary(&auth_url).into_response();
    let headers = response.headers_mut();
    headers.append(header::SET_COOKIE, state_cookie.parse().unwrap());
    headers.append(header::SET_COOKIE, pkce_cookie.parse().unwrap());

    Ok(response)
}

pub async fn callback<C: AuthContext>(
    State(ctx): State<C>,
    axum::extract::Path(provider): axum::extract::Path<String>,
    Query(params): Query<HashMap<String, String>>,
    cookies: tower_cookies::Cookies,
) -> Result<Response, AuthError> {
    let code = params.get("code")
        .ok_or_else(|| AuthError::bad_request("missing code parameter"))?;
    let state = params.get("state")
        .ok_or_else(|| AuthError::bad_request("missing state parameter"))?;

    let expected_state = cookies.get("oauth_state")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AuthError::bad_request("missing oauth_state cookie"))?;

    if expected_state != *state {
        return Err(AuthError::bad_request("invalid OAuth state"));
    }

    let pkce_verifier = cookies.get("oauth_pkce")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AuthError::bad_request("missing oauth_pkce cookie"))?;

    let clear_state = "oauth_state=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    let clear_pkce = "oauth_pkce=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";

    let tokens = services::oauth_service::handle_callback(
        &ctx,
        &provider,
        code,
        state,
        &pkce_verifier,
    ).await?;

    let mut response = token_response(&ctx, &tokens, &format!("authenticated via {provider}"));
    let headers = response.headers_mut();
    headers.append(header::SET_COOKIE, clear_state.parse().unwrap());
    headers.append(header::SET_COOKIE, clear_pkce.parse().unwrap());

    Ok(response)
}
