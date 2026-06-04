use rok_validate::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct SendOtpRequest {
    #[validate(required, email)]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyOtpRequest {
    #[validate(required, email)]
    pub email: String,
    #[validate(required)]
    pub code: String,
}
