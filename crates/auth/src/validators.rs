use axum::body::Body;
use axum::extract::FromRequest;
use axum::http::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use validator::Validate;

// ── Password validation ──────────────────────────────────────────

fn validate_password(pw: &str) -> Result<(), validator::ValidationError> {
    if !pw.chars().any(|c| c.is_uppercase()) {
        let mut err = validator::ValidationError::new("password_no_uppercase");
        err.message = Some("must contain at least one uppercase letter".into());
        return Err(err);
    }
    if !pw.chars().any(|c| c.is_lowercase()) {
        let mut err = validator::ValidationError::new("password_no_lowercase");
        err.message = Some("must contain at least one lowercase letter".into());
        return Err(err);
    }
    if !pw.chars().any(|c| c.is_ascii_digit()) {
        let mut err = validator::ValidationError::new("password_no_digit");
        err.message = Some("must contain at least one digit".into());
        return Err(err);
    }
    if !pw.chars().any(|c| !c.is_alphanumeric() && !c.is_whitespace()) {
        let mut err = validator::ValidationError::new("password_no_symbol");
        err.message = Some("must contain at least one special character".into());
        return Err(err);
    }
    Ok(())
}

// ── Auth request DTOs ────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128), custom(function = "validate_password"))]
    pub password: String,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(length(min = 8, max = 128), custom(function = "validate_password"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RefreshRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

// ── OTP request DTOs ─────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct SendOtpRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyOtpRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub code: String,
}

// ── Magic link DTOs ──────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct MagicLinkRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct MagicLinkVerifyRequest {
    #[validate(length(min = 1))]
    pub token: String,
}

// ── Login OTP DTOs ───────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct LoginOtpSendRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginOtpVerifyRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub code: String,
}

// ── Username/email login DTO ─────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct UsernameLoginRequest {
    #[validate(length(min = 1, max = 255))]
    pub identifier: String,
    #[validate(length(min = 1))]
    pub password: String,
}

// ── ValidatedJson extractor ──────────────────────────────────────

pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ValidatedJsonRejection;

    async fn from_request(req: Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(body) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ValidatedJsonRejection::Deserialization(e.to_string()))?;

        body.validate()
            .map_err(|e| ValidatedJsonRejection::Validation(e.to_string()))?;

        Ok(ValidatedJson(body))
    }
}

pub enum ValidatedJsonRejection {
    Deserialization(String),
    Validation(String),
}

impl IntoResponse for ValidatedJsonRejection {
    fn into_response(self) -> Response {
        let msg = match self {
            Self::Deserialization(m) => m,
            Self::Validation(m) => m,
        };
        let body = serde_json::json!({
            "status": "error",
            "error": { "code": "UNPROCESSABLE_ENTITY", "message": msg }
        });
        (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(body)).into_response()
    }
}
