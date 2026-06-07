pub mod auth;
pub mod otp;
pub mod user;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::response::ErrorBody;

pub fn validate<T: DeserializeOwned + Validate>(
    body: T,
) -> Result<T, ValidationRejection> {
    body.validate().map_err(ValidationRejection::ValidationError)?;
    Ok(body)
}

#[derive(Debug)]
pub enum ValidationRejection {
    ValidationError(validator::ValidationErrors),
}

impl IntoResponse for ValidationRejection {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::ValidationError(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorBody {
                    error: "E_VALIDATION".to_string(),
                    message: errors.to_string(),
                }),
            ),
        };
        (status, body).into_response()
    }
}
