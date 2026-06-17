use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::Request;
use axum::http::header;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tower::{Layer, Service};

use crate::primitives::verify_token;

#[derive(Clone, Debug, PartialEq)]
pub enum AuthStrategy {
    Bearer,
    Cookie,
}

#[derive(Clone)]
pub struct JwtAuthLayer {
    secret: Arc<String>,
    strategy: AuthStrategy,
}

impl JwtAuthLayer {
    pub fn new(secret: String) -> Self {
        Self {
            secret: Arc::new(secret),
            strategy: AuthStrategy::Bearer,
        }
    }

    pub fn with_strategy(secret: String, strategy: AuthStrategy) -> Self {
        Self {
            secret: Arc::new(secret),
            strategy,
        }
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthService {
            inner,
            secret: self.secret.clone(),
            strategy: self.strategy.clone(),
        }
    }
}

#[derive(Clone)]
pub struct JwtAuthService<S> {
    inner: S,
    secret: Arc<String>,
    strategy: AuthStrategy,
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
        let strategy = self.strategy.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Try Bearer header first
            let token = req
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(|s| s.to_string());

            // If no Bearer, try cookie (only in cookie strategy)
            let token = if token.is_some() {
                token
            } else if strategy == AuthStrategy::Cookie {
                req.headers()
                    .get(header::COOKIE)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| {
                        v.split(';')
                            .map(|c| c.trim())
                            .find(|c| c.starts_with("access_token="))
                            .and_then(|c| c.get("access_token=".len()..))
                            .map(|s| s.to_string())
                    })
            } else {
                None
            };

            match token {
                Some(token) => match verify_token(&token, &secret) {
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
