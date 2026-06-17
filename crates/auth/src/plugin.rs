use api_core::response::ApiResponse;
use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use std::collections::HashMap;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

use crate::context::AuthContext;
use crate::error::AuthError;
use crate::extractors::AuthUser;
use crate::middleware::AuthStrategy;
use crate::primitives::TokenPair;
use crate::services;
use crate::validators::ValidatedJson;
use crate::validators::{
    ForgotPasswordRequest, LoginOtpSendRequest, LoginOtpVerifyRequest,
    LoginRequest, MagicLinkRequest, RefreshRequest, RegisterRequest, ResetPasswordRequest,
    SendOtpRequest, TwoFactorDisableRequest, TwoFactorVerifyRequest, VerifyOtpRequest,
};

// ─── Plugin Builder ───────────────────────────────────────

pub struct AuthPlugin {
    pub(crate) magic_link: bool,
    pub(crate) login_otp: bool,
    pub(crate) totp_2fa: bool,
    pub(crate) sessions: bool,
    pub(crate) google: bool,
    pub(crate) github: bool,
}

impl AuthPlugin {
    pub fn builder() -> AuthPluginBuilder {
        AuthPluginBuilder {
            magic_link: false,
            login_otp: false,
            totp_2fa: false,
            sessions: false,
            google: false,
            github: false,
        }
    }

    pub fn public_routes<C: AuthContext>(&self) -> Router<C> {
        let mut routes = Router::new()
            .route("/register", post(register::<C>))
            .route("/login", post(login::<C>))
            .route("/refresh", post(refresh_handler::<C>))
            .route("/forgot-password", post(forgot_password::<C>))
            .route("/reset-password", post(reset_password::<C>));

        if self.magic_link {
            routes = routes
                .route("/magic-link", post(magic_link_request::<C>))
                .route("/magic-link/verify", get(magic_link_verify::<C>));
        }

        if self.login_otp {
            routes = routes
                .route("/otp/login/send", post(login_otp_send::<C>))
                .route("/otp/login/verify", post(login_otp_verify::<C>));
        }

        if self.google {
            routes = routes
                .route("/oauth/google/redirect", get(oauth_redirect::<C>))
                .route("/oauth/google/callback", get(oauth_callback::<C>));
        }

        if self.github {
            routes = routes
                .route("/oauth/github/redirect", get(oauth_redirect::<C>))
                .route("/oauth/github/callback", get(oauth_callback::<C>));
        }

        // Apply rate limiting to strict routes
        let strict_config = GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(5)
            .finish()
            .unwrap();

        let strict_routes = Router::new()
            .route("/register", post(register::<C>))
            .route("/login", post(login::<C>))
            .layer(GovernorLayer::new(strict_config));

        // Merge: strict routes override the unthrottled ones
        routes.merge(strict_routes)
    }

    pub fn protected_routes<C: AuthContext>(&self) -> Router<C> {
        let mut routes = Router::new()
            .route("/logout", post(logout::<C>));

        if self.totp_2fa {
            routes = routes
                .route("/2fa/enable", post(two_factor_enable::<C>))
                .route("/2fa/verify", post(two_factor_verify::<C>))
                .route("/2fa/disable", post(two_factor_disable::<C>));
        }

        if self.sessions {
            routes = routes
                .route("/me/sessions", get(session_list::<C>))
                .route("/me/sessions/{id}", axum::routing::delete(session_revoke::<C>))
                .route("/me/sessions", axum::routing::delete(session_revoke_all::<C>));
        }

        // OTP routes (registration verification)
        routes = routes
            .route("/otp/send", post(otp_send::<C>))
            .route("/otp/verify", post(otp_verify::<C>));

        routes
    }
}

pub struct AuthPluginBuilder {
    magic_link: bool,
    login_otp: bool,
    totp_2fa: bool,
    sessions: bool,
    google: bool,
    github: bool,
}

impl AuthPluginBuilder {
    pub fn magic_link(mut self) -> Self { self.magic_link = true; self }
    pub fn login_otp(mut self) -> Self { self.login_otp = true; self }
    pub fn totp_2fa(mut self) -> Self { self.totp_2fa = true; self }
    pub fn sessions(mut self) -> Self { self.sessions = true; self }
    pub fn google(mut self) -> Self { self.google = true; self }
    pub fn github(mut self) -> Self { self.github = true; self }

    pub fn build(self) -> AuthPlugin {
        AuthPlugin {
            magic_link: self.magic_link,
            login_otp: self.login_otp,
            totp_2fa: self.totp_2fa,
            sessions: self.sessions,
            google: self.google,
            github: self.github,
        }
    }
}

// ─── Token Response Helper ────────────────────────────────

fn token_response<C: AuthContext>(ctx: &C, tokens: &TokenPair, message: &str) -> Response {
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

// ─── Handlers ─────────────────────────────────────────────

async fn register<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<RegisterRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::auth_service::register(&ctx, &body.email, &body.password, &body.name).await?;
    Ok(token_response(&ctx, &tokens, "registered"))
}

async fn login<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<LoginRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::auth_service::login(&ctx, &body.email, &body.password).await?;
    Ok(token_response(&ctx, &tokens, "logged in"))
}

async fn refresh_handler<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<RefreshRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::auth_service::refresh(&ctx, &body.refresh_token).await?;
    Ok(token_response(&ctx, &tokens, "refreshed"))
}

async fn logout<C: AuthContext>(
    State(ctx): State<C>,
    _user: AuthUser,
) -> Response {
    if ctx.config().auth_strategy == AuthStrategy::Cookie {
        let body = ApiResponse::ok(serde_json::json!({ "message": "logged out" }));
        let mut response = body.into_response();
        let headers = response.headers_mut();
        let clear_access = "access_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
        let clear_refresh = "refresh_token=; Path=/api/v1/auth/refresh; HttpOnly; SameSite=Lax; Max-Age=0";
        headers.append(header::SET_COOKIE, clear_access.parse().unwrap());
        headers.append(header::SET_COOKIE, clear_refresh.parse().unwrap());
        response
    } else {
        ApiResponse::ok(serde_json::json!({ "message": "logged out" })).into_response()
    }
}

async fn forgot_password<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<ForgotPasswordRequest>,
) -> Result<ApiResponse, AuthError> {
    services::auth_service::forgot_password(&ctx, &body.email).await?;
    Ok(ApiResponse::message("reset link sent"))
}

async fn reset_password<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<ResetPasswordRequest>,
) -> Result<ApiResponse, AuthError> {
    services::auth_service::reset_password(&ctx, &body.token, &body.password).await?;
    Ok(ApiResponse::message("password reset"))
}

// ── Magic Link ────────────────────────────────────────────

async fn magic_link_request<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<MagicLinkRequest>,
) -> Result<ApiResponse, AuthError> {
    services::magic_link_service::request_magic_link(&ctx, &body.email).await?;
    Ok(ApiResponse::message("magic link sent"))
}

async fn magic_link_verify<C: AuthContext>(
    State(ctx): State<C>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, AuthError> {
    let token = params.get("token")
        .ok_or_else(|| AuthError::bad_request("missing token parameter"))?;
    let tokens = services::magic_link_service::verify_magic_link(&ctx, token).await?;
    Ok(token_response(&ctx, &tokens, "authenticated via magic link"))
}

// ── Login OTP ─────────────────────────────────────────────

async fn login_otp_send<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<LoginOtpSendRequest>,
) -> Result<ApiResponse, AuthError> {
    services::login_otp_service::send_login_otp(&ctx, &body.email).await?;
    Ok(ApiResponse::message("login code sent"))
}

async fn login_otp_verify<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<LoginOtpVerifyRequest>,
) -> Result<Response, AuthError> {
    let tokens = services::login_otp_service::verify_login_otp(&ctx, &body.email, &body.code).await?;
    Ok(token_response(&ctx, &tokens, "authenticated via OTP"))
}

// ── OTP (Registration Verification) ───────────────────────

async fn otp_send<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<SendOtpRequest>,
) -> Result<ApiResponse, AuthError> {
    services::otp_service::send(&ctx, &body.email).await?;
    Ok(ApiResponse::message("verification email sent"))
}

async fn otp_verify<C: AuthContext>(
    State(ctx): State<C>,
    ValidatedJson(body): ValidatedJson<VerifyOtpRequest>,
) -> Result<ApiResponse, AuthError> {
    services::otp_service::verify(&ctx, &body.email, &body.code).await?;
    Ok(ApiResponse::message("email verified"))
}

// ── Two-Factor Auth ───────────────────────────────────────

async fn two_factor_enable<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
) -> Result<ApiResponse, AuthError> {
    let result = services::two_factor_service::enable(&ctx, &user.user_id).await?;
    Ok(ApiResponse::ok(serde_json::json!({
        "secret": result.secret,
        "otpauth_url": result.otpauth_url,
        "backup_codes": result.backup_codes,
    })))
}

async fn two_factor_verify<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
    ValidatedJson(body): ValidatedJson<TwoFactorVerifyRequest>,
) -> Result<ApiResponse, AuthError> {
    services::two_factor_service::verify_and_activate(&ctx, &user.user_id, &body.code).await?;
    Ok(ApiResponse::message("2FA enabled"))
}

async fn two_factor_disable<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
    ValidatedJson(body): ValidatedJson<TwoFactorDisableRequest>,
) -> Result<ApiResponse, AuthError> {
    services::two_factor_service::disable(&ctx, &user.user_id, &body.password, &body.code).await?;
    Ok(ApiResponse::message("2FA disabled"))
}

// ── Sessions ──────────────────────────────────────────────

async fn session_list<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
) -> Result<ApiResponse, AuthError> {
    let sessions: Vec<crate::session::Session> = services::session_service::list_for_user(&ctx, &user.user_id).await?;
    Ok(ApiResponse::data("sessions", sessions))
}

async fn session_revoke<C: AuthContext>(
    State(ctx): State<C>,
    _user: AuthUser,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<ApiResponse, AuthError> {
    services::session_service::revoke(&ctx, &session_id).await?;
    Ok(ApiResponse::message("session revoked"))
}

async fn session_revoke_all<C: AuthContext>(
    State(ctx): State<C>,
    user: AuthUser,
) -> Result<ApiResponse, AuthError> {
    services::session_service::revoke_all_for_user(&ctx, &user.user_id).await?;
    Ok(ApiResponse::message("all sessions revoked"))
}

// ── OAuth (Social Login) ──────────────────────────────────

async fn oauth_redirect<C: AuthContext>(
    State(ctx): State<C>,
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<axum::response::Redirect, AuthError> {
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

    // Store state + pkce_verifier in cookies for callback verification
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

    Ok(axum::response::Redirect::temporary(&auth_url))
}

async fn oauth_callback<C: AuthContext>(
    State(ctx): State<C>,
    axum::extract::Path(provider): axum::extract::Path<String>,
    Query(params): Query<HashMap<String, String>>,
    cookies: tower_cookies::Cookies,
) -> Result<Response, AuthError> {
    let code = params.get("code")
        .ok_or_else(|| AuthError::bad_request("missing code parameter"))?;
    let state = params.get("state")
        .ok_or_else(|| AuthError::bad_request("missing state parameter"))?;

    // Verify state from cookie
    let expected_state = cookies.get("oauth_state")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AuthError::bad_request("missing oauth_state cookie"))?;

    if expected_state != *state {
        return Err(AuthError::bad_request("invalid OAuth state"));
    }

    // Get PKCE verifier from cookie
    let pkce_verifier = cookies.get("oauth_pkce")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AuthError::bad_request("missing oauth_pkce cookie"))?;

    // Clear OAuth cookies
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
