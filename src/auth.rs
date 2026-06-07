use std::time::Duration;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub roles: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn generate_token_pair(
    user_id: &str,
    roles: &str,
    secret: &str,
    token_ttl: Duration,
    refresh_ttl: Duration,
) -> Result<TokenPair, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;

    let access_claims = Claims {
        sub: user_id.to_string(),
        exp: now + token_ttl.as_secs() as usize,
        iat: now,
        roles: roles.to_string(),
    };

    let refresh_claims = Claims {
        sub: user_id.to_string(),
        exp: now + refresh_ttl.as_secs() as usize,
        iat: now,
        roles: roles.to_string(),
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(TokenPair {
        access_token,
        refresh_token,
    })
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    #[allow(dead_code)]
    pub claims: Claims,
    pub user_id: String,
    pub roles: Vec<String>,
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthRejection::MissingToken)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AuthRejection::InvalidScheme)?;

        let claims = verify_token(token, &app_state.config.auth_secret)
            .map_err(|_| AuthRejection::InvalidToken)?;

        let user_id = claims.sub.clone();
        let roles: Vec<String> = claims.roles.split(',').map(|s| s.trim().to_string()).collect();

        Ok(AuthUser {
            claims,
            user_id,
            roles,
        })
    }
}

#[allow(dead_code)]
pub struct AdminOnly(pub AuthUser);

impl<S> FromRequestParts<S> for AdminOnly
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.roles.iter().any(|r| r == "admin") {
            return Err(AuthRejection::Forbidden);
        }
        Ok(AdminOnly(user))
    }
}

#[derive(Debug)]
pub enum AuthRejection {
    MissingToken,
    InvalidScheme,
    InvalidToken,
    Forbidden,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let (status, code, msg) = match self {
            Self::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "E_MISSING_TOKEN",
                "missing authorization header",
            ),
            Self::InvalidScheme => (
                StatusCode::UNAUTHORIZED,
                "E_INVALID_SCHEME",
                "invalid authorization scheme, use Bearer",
            ),
            Self::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "E_INVALID_TOKEN",
                "invalid or expired token",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "E_FORBIDDEN",
                "insufficient permissions",
            ),
        };
        let body = serde_json::json!({
            "error": code,
            "message": msg,
        });
        (status, axum::Json(body)).into_response()
    }
}
