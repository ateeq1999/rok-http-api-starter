use rok_validate::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(required, email)]
    pub email: String,
    #[validate(required, min = 8, max = 128)]
    pub password: String,
    #[validate(required, max = 255)]
    pub name: String,
    pub roles: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub name: Option<String>,
    pub roles: Option<String>,
}
