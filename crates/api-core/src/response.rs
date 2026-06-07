use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;

pub struct ApiResponse {
    status: StatusCode,
    body: Value,
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl ApiResponse {
    pub fn ok(data: Value) -> Self {
        Self { status: StatusCode::OK, body: serde_json::json!({ "data": data }) }
    }

    pub fn created(data: Value) -> Self {
        Self { status: StatusCode::CREATED, body: serde_json::json!({ "data": data }) }
    }

    pub fn no_content() -> Self {
        Self { status: StatusCode::NO_CONTENT, body: serde_json::json!(null) }
    }

    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        let body = serde_json::json!({
            "error": {
                "code": code.code_str(),
                "message": message.into(),
            }
        });
        Self { status: code.status(), body }
    }

    pub fn paginated(data: Value, total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = (total as f64 / per_page as f64).ceil() as i64;
        Self {
            status: StatusCode::OK,
            body: serde_json::json!({
                "data": data,
                "pagination": {
                    "total": total,
                    "page": page,
                    "per_page": per_page,
                    "total_pages": total_pages,
                }
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum ErrorCode {
    Ok, Created, Accepted, NonAuthoritativeInfo, NoContent,
    ResetContent, PartialContent, MultiStatus, AlreadyReported, ImUsed,
    MultipleChoices, MovedPermanently, Found, SeeOther, NotModified,
    UseProxy, TemporaryRedirect, PermanentRedirect,
    BadRequest, Unauthorized, PaymentRequired, Forbidden, NotFound,
    MethodNotAllowed, NotAcceptable, ProxyAuthRequired, RequestTimeout, Conflict,
    Gone, LengthRequired, PreconditionFailed, PayloadTooLarge, UriTooLong,
    UnsupportedMediaType, RangeNotSatisfiable, ExpectationFailed, ImATeapot,
    MisdirectedRequest, UnprocessableEntity, Locked, FailedDependency, TooEarly,
    UpgradeRequired, PreconditionRequired, TooManyRequests, HeaderFieldsTooLarge,
    UnavailableForLegalReasons,
    InternalServerError, NotImplemented, BadGateway, ServiceUnavailable,
    GatewayTimeout, HttpVersionNotSupported, VariantAlsoNegotiates,
    InsufficientStorage, LoopDetected, NotExtended, NetworkAuthRequired,
}

impl ErrorCode {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Ok => StatusCode::OK,
            Self::Created => StatusCode::CREATED,
            Self::Accepted => StatusCode::ACCEPTED,
            Self::NonAuthoritativeInfo => StatusCode::NON_AUTHORITATIVE_INFORMATION,
            Self::NoContent => StatusCode::NO_CONTENT,
            Self::ResetContent => StatusCode::RESET_CONTENT,
            Self::PartialContent => StatusCode::PARTIAL_CONTENT,
            Self::MultiStatus => StatusCode::MULTI_STATUS,
            Self::AlreadyReported => StatusCode::ALREADY_REPORTED,
            Self::ImUsed => StatusCode::IM_USED,
            Self::MultipleChoices => StatusCode::MULTIPLE_CHOICES,
            Self::MovedPermanently => StatusCode::MOVED_PERMANENTLY,
            Self::Found => StatusCode::FOUND,
            Self::SeeOther => StatusCode::SEE_OTHER,
            Self::NotModified => StatusCode::NOT_MODIFIED,
            Self::UseProxy => StatusCode::USE_PROXY,
            Self::TemporaryRedirect => StatusCode::TEMPORARY_REDIRECT,
            Self::PermanentRedirect => StatusCode::PERMANENT_REDIRECT,
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::PaymentRequired => StatusCode::PAYMENT_REQUIRED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Self::NotAcceptable => StatusCode::NOT_ACCEPTABLE,
            Self::ProxyAuthRequired => StatusCode::PROXY_AUTHENTICATION_REQUIRED,
            Self::RequestTimeout => StatusCode::REQUEST_TIMEOUT,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Gone => StatusCode::GONE,
            Self::LengthRequired => StatusCode::LENGTH_REQUIRED,
            Self::PreconditionFailed => StatusCode::PRECONDITION_FAILED,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::UriTooLong => StatusCode::URI_TOO_LONG,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::RangeNotSatisfiable => StatusCode::RANGE_NOT_SATISFIABLE,
            Self::ExpectationFailed => StatusCode::EXPECTATION_FAILED,
            Self::ImATeapot => StatusCode::IM_A_TEAPOT,
            Self::MisdirectedRequest => StatusCode::MISDIRECTED_REQUEST,
            Self::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Locked => StatusCode::LOCKED,
            Self::FailedDependency => StatusCode::FAILED_DEPENDENCY,
            Self::TooEarly => StatusCode::TOO_EARLY,
            Self::UpgradeRequired => StatusCode::UPGRADE_REQUIRED,
            Self::PreconditionRequired => StatusCode::PRECONDITION_REQUIRED,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::HeaderFieldsTooLarge => StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
            Self::UnavailableForLegalReasons => StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
            Self::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotImplemented => StatusCode::NOT_IMPLEMENTED,
            Self::BadGateway => StatusCode::BAD_GATEWAY,
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::GatewayTimeout => StatusCode::GATEWAY_TIMEOUT,
            Self::HttpVersionNotSupported => StatusCode::HTTP_VERSION_NOT_SUPPORTED,
            Self::VariantAlsoNegotiates => StatusCode::VARIANT_ALSO_NEGOTIATES,
            Self::InsufficientStorage => StatusCode::INSUFFICIENT_STORAGE,
            Self::LoopDetected => StatusCode::LOOP_DETECTED,
            Self::NotExtended => StatusCode::NOT_EXTENDED,
            Self::NetworkAuthRequired => StatusCode::NETWORK_AUTHENTICATION_REQUIRED,
        }
    }

    pub fn code_str(&self) -> &'static str {
        match self {
            Self::Ok => "E_OK",
            Self::Created => "E_CREATED",
            Self::Accepted => "E_ACCEPTED",
            Self::NonAuthoritativeInfo => "E_NON_AUTHORITATIVE_INFO",
            Self::NoContent => "E_NO_CONTENT",
            Self::ResetContent => "E_RESET_CONTENT",
            Self::PartialContent => "E_PARTIAL_CONTENT",
            Self::MultiStatus => "E_MULTI_STATUS",
            Self::AlreadyReported => "E_ALREADY_REPORTED",
            Self::ImUsed => "E_IM_USED",
            Self::MultipleChoices => "E_MULTIPLE_CHOICES",
            Self::MovedPermanently => "E_MOVED_PERMANENTLY",
            Self::Found => "E_FOUND",
            Self::SeeOther => "E_SEE_OTHER",
            Self::NotModified => "E_NOT_MODIFIED",
            Self::UseProxy => "E_USE_PROXY",
            Self::TemporaryRedirect => "E_TEMPORARY_REDIRECT",
            Self::PermanentRedirect => "E_PERMANENT_REDIRECT",
            Self::BadRequest => "E_BAD_REQUEST",
            Self::Unauthorized => "E_UNAUTHORIZED",
            Self::PaymentRequired => "E_PAYMENT_REQUIRED",
            Self::Forbidden => "E_FORBIDDEN",
            Self::NotFound => "E_NOT_FOUND",
            Self::MethodNotAllowed => "E_METHOD_NOT_ALLOWED",
            Self::NotAcceptable => "E_NOT_ACCEPTABLE",
            Self::ProxyAuthRequired => "E_PROXY_AUTH_REQUIRED",
            Self::RequestTimeout => "E_REQUEST_TIMEOUT",
            Self::Conflict => "E_CONFLICT",
            Self::Gone => "E_GONE",
            Self::LengthRequired => "E_LENGTH_REQUIRED",
            Self::PreconditionFailed => "E_PRECONDITION_FAILED",
            Self::PayloadTooLarge => "E_PAYLOAD_TOO_LARGE",
            Self::UriTooLong => "E_URI_TOO_LONG",
            Self::UnsupportedMediaType => "E_UNSUPPORTED_MEDIA_TYPE",
            Self::RangeNotSatisfiable => "E_RANGE_NOT_SATISFIABLE",
            Self::ExpectationFailed => "E_EXPECTATION_FAILED",
            Self::ImATeapot => "E_IM_A_TEAPOT",
            Self::MisdirectedRequest => "E_MISDIRECTED_REQUEST",
            Self::UnprocessableEntity => "E_UNPROCESSABLE_ENTITY",
            Self::Locked => "E_LOCKED",
            Self::FailedDependency => "E_FAILED_DEPENDENCY",
            Self::TooEarly => "E_TOO_EARLY",
            Self::UpgradeRequired => "E_UPGRADE_REQUIRED",
            Self::PreconditionRequired => "E_PRECONDITION_REQUIRED",
            Self::TooManyRequests => "E_TOO_MANY_REQUESTS",
            Self::HeaderFieldsTooLarge => "E_HEADER_FIELDS_TOO_LARGE",
            Self::UnavailableForLegalReasons => "E_UNAVAILABLE_FOR_LEGAL_REASONS",
            Self::InternalServerError => "E_INTERNAL_SERVER_ERROR",
            Self::NotImplemented => "E_NOT_IMPLEMENTED",
            Self::BadGateway => "E_BAD_GATEWAY",
            Self::ServiceUnavailable => "E_SERVICE_UNAVAILABLE",
            Self::GatewayTimeout => "E_GATEWAY_TIMEOUT",
            Self::HttpVersionNotSupported => "E_HTTP_VERSION_NOT_SUPPORTED",
            Self::VariantAlsoNegotiates => "E_VARIANT_ALSO_NEGOTIATES",
            Self::InsufficientStorage => "E_INSUFFICIENT_STORAGE",
            Self::LoopDetected => "E_LOOP_DETECTED",
            Self::NotExtended => "E_NOT_EXTENDED",
            Self::NetworkAuthRequired => "E_NETWORK_AUTH_REQUIRED",
        }
    }
}
