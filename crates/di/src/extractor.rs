use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::Container;

/// Axum extractor pulling a provider out of the DI container reachable from `S` via `FromRef`.
///
/// Because the whole provider graph is constructed eagerly at startup (see [`crate::Module`]),
/// this should never actually reject in a running server — the only way it fires is a handler
/// signature referencing a type nothing ever registered (a typo, or a genuine wiring bug),
/// which a single request to that route will catch.
pub struct Injected<T: ?Sized>(pub Arc<T>);

impl<S, T> FromRequestParts<S> for Injected<T>
where
    Container: FromRef<S>,
    S: Send + Sync,
    T: ?Sized + Send + Sync + 'static,
{
    type Rejection = DiRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let State(container) = State::<Container>::from_request_parts(parts, state)
            .await
            .expect("infallible: State<T> extraction cannot fail");
        container
            .get::<T>()
            .map(Injected)
            .ok_or_else(DiRejection::not_registered::<T>)
    }
}

#[derive(Debug)]
pub struct DiRejection {
    type_name: &'static str,
}

impl DiRejection {
    fn not_registered<T: ?Sized>() -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
        }
    }
}

impl std::fmt::Display for DiRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` is not registered in the DI container", self.type_name)
    }
}

impl std::error::Error for DiRejection {}

impl IntoResponse for DiRejection {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
