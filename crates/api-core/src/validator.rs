use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::response::{ApiResponse, ErrorCode};

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
        match self {
            Self::ValidationError(errors) => {
                ApiResponse::error(ErrorCode::UnprocessableEntity, errors.to_string())
                    .into_response()
            }
        }
    }
}
