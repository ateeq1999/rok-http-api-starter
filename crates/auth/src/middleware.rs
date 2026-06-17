use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tower::{Layer, Service};

use crate::primitives::verify_token;

#[derive(Clone)]
pub struct JwtAuthLayer {
    secret: Arc<String>,
}

impl JwtAuthLayer {
    pub fn new(secret: String) -> Self {
        Self {
            secret: Arc::new(secret),
        }
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthService {
            inner,
            secret: self.secret.clone(),
        }
    }
}

#[derive(Clone)]
pub struct JwtAuthService<S> {
    inner: S,
    secret: Arc<String>,
}

impl<S> Service<Request> for JwtAuthService<S>
where
    S: Service<Request, Response = Response> + Send + Clone + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let secret = self.secret.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let token = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            match token {
                Some(token) => match verify_token(token, &secret) {
                    Ok(claims) => {
                        let mut req = req;
                        req.extensions_mut().insert(claims);
                        Ok(inner.call(req).await?)
                    }
                    Err(_) => Ok(jwt_rejection("invalid or expired token")),
                },
                None => Ok(jwt_rejection("missing authorization header")),
            }
        })
    }
}

fn jwt_rejection(msg: &str) -> Response {
    let body = serde_json::json!({
        "status": "error",
        "error": { "code": "UNAUTHORIZED", "message": msg }
    });
    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}
