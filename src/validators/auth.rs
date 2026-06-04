use rok_validate::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(required, email)]
    pub email: String,
    #[validate(required, min = 8, max = 128)]
    pub password: String,
    #[validate(required, max = 255)]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(required, email)]
    pub email: String,
    #[validate(required)]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(required, email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    #[validate(required)]
    pub token: String,
    #[validate(required, min = 8, max = 128)]
    pub password: String,
}
