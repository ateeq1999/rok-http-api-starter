use async_trait::async_trait;
use axum::response::{IntoResponse, Redirect, Response};

use rok_auth::social::{SocialAuthHooks, SocialError, SocialUser};

pub struct AppSocialHooks;

#[async_trait]
impl SocialAuthHooks for AppSocialHooks {
    async fn upsert_user(&self, social_user: SocialUser) -> Result<Response, SocialError> {
        let email = social_user
            .email
            .ok_or_else(|| SocialError::Hook("email required".into()))?;
        let name = social_user.name.unwrap_or_else(|| "User".into());

        tracing::info!("social login: {} ({})", email, social_user.provider);

        Ok(Redirect::to(&format!("/auth/login?social_provider={}&email={}", social_user.provider, email)).into_response())
    }
}
