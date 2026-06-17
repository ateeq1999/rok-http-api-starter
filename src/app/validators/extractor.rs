use axum::body::Body;
use axum::extract::FromRequest;
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use validator::Validate;

use api_core::response::{ApiResponse, ErrorCode};

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
        ApiResponse::error(ErrorCode::UnprocessableEntity, msg).into_response()
    }
}
