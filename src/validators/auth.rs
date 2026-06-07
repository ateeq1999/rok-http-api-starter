use serde::Deserialize;
use validator::Validate;

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
