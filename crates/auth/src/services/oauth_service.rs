use crate::context::{AuthContext, OAuthProviderConfig};
use crate::error::AuthError;
use crate::primitives;
use crate::primitives::TokenPair;

#[derive(Debug, Clone, serde::Serialize)]
pub struct OAuthState {
    pub provider: String,
    pub state: String,
    pub pkce_verifier: String,
}

pub fn start_authorization(
    provider: &str,
    config: &OAuthProviderConfig,
    _base_url: &str,
) -> Result<(String, String, String), AuthError> {
    use oauth2::{
        AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
        RedirectUrl, TokenUrl,
    };

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = match provider {
        "google" => {
            let client = oauth2::basic::BasicClient::new(ClientId::new(config.client_id.clone()))
                .set_client_secret(ClientSecret::new(config.client_secret.clone()))
                .set_auth_uri(AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string()).unwrap())
                .set_token_uri(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap())
                .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone()).unwrap());
            client
                .authorize_url(CsrfToken::new_random)
                .add_scope(oauth2::Scope::new("email".to_string()))
                .add_scope(oauth2::Scope::new("profile".to_string()))
                .set_pkce_challenge(pkce_challenge)
                .url()
        }
        "github" => {
            let client = oauth2::basic::BasicClient::new(ClientId::new(config.client_id.clone()))
                .set_client_secret(ClientSecret::new(config.client_secret.clone()))
                .set_auth_uri(AuthUrl::new("https://github.com/login/oauth/authorize".to_string()).unwrap())
                .set_token_uri(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap())
                .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone()).unwrap());
            client
                .authorize_url(CsrfToken::new_random)
                .add_scope(oauth2::Scope::new("read:user".to_string()))
                .add_scope(oauth2::Scope::new("user:email".to_string()))
                .set_pkce_challenge(pkce_challenge)
                .url()
        }
        _ => return Err(AuthError::bad_request(format!("unsupported provider: {provider}"))),
    };

    Ok((auth_url.to_string(), csrf_token.secret().clone(), pkce_verifier.secret().clone()))
}

pub async fn handle_callback<C: AuthContext>(
    ctx: &C,
    provider: &str,
    code: &str,
    _state: &str,
    pkce_verifier: &str,
) -> Result<TokenPair, AuthError> {
    use oauth2::{
        ClientId, ClientSecret, TokenUrl, TokenResponse,
    };

    let config = ctx.config();
    let provider_config = match provider {
        "google" => config.google.as_ref().ok_or_else(|| AuthError::internal("Google OAuth not configured"))?,
        "github" => config.github.as_ref().ok_or_else(|| AuthError::internal("GitHub OAuth not configured"))?,
        _ => return Err(AuthError::bad_request(format!("unsupported provider: {provider}"))),
    };

    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AuthError::internal(format!("failed to build HTTP client: {e}")))?;

    // Exchange code for tokens
    let token_result = match provider {
        "google" => {
            let client = oauth2::basic::BasicClient::new(ClientId::new(provider_config.client_id.clone()))
                .set_client_secret(ClientSecret::new(provider_config.client_secret.clone()))
                .set_token_uri(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap());
            client
                .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
                .set_pkce_verifier(oauth2::PkceCodeVerifier::new(pkce_verifier.to_string()))
                .request_async(&http_client)
                .await
                .map_err(|e| AuthError::internal(format!("token exchange failed: {e}")))?
        }
        "github" => {
            let client = oauth2::basic::BasicClient::new(ClientId::new(provider_config.client_id.clone()))
                .set_client_secret(ClientSecret::new(provider_config.client_secret.clone()))
                .set_token_uri(TokenUrl::new("https://github.com/login/oauth/access_token".to_string()).unwrap());
            client
                .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
                .set_pkce_verifier(oauth2::PkceCodeVerifier::new(pkce_verifier.to_string()))
                .request_async(&http_client)
                .await
                .map_err(|e| AuthError::internal(format!("token exchange failed: {e}")))?
        }
        _ => unreachable!(),
    };

    let access_token = token_result.access_token().secret().clone();

    // Fetch user info from provider
    let user_info = fetch_user_info(provider, &access_token).await?;

    // Find or create user
    let email = user_info.email.clone()
        .ok_or_else(|| AuthError::internal("provider did not return email"))?;

    let finder = ctx.user_finder();
    let user = match finder.find_by_email(&email).await? {
        Some(user) => user,
        None => {
            // Create new user
            let name = user_info.name.clone().unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());
            let _avatar = user_info.avatar_url.clone();
            finder.create_user(&[
                ("id", &primitives::generate_id()),
                ("email", &email),
                ("password_hash", ""), // No password for OAuth users
                ("name", &name),
                ("roles", "user"),
            ]).await?
        }
    };

    // Upsert account record
    let provider_account_id = user_info.id.clone().unwrap_or_default();
    sqlx::query(
        "INSERT INTO accounts (id, user_id, provider, provider_account_id, access_token, refresh_token, provider_user_data)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (provider, provider_account_id) DO UPDATE SET
           access_token = EXCLUDED.access_token,
           refresh_token = EXCLUDED.refresh_token,
           updated_at = NOW()",
    )
    .bind(primitives::generate_id())
    .bind(&user.id)
    .bind(provider)
    .bind(&provider_account_id)
    .bind(&access_token)
    .bind(token_result.refresh_token().map(|t| t.secret().clone()))
    .bind(serde_json::json!({"raw": user_info}))
    .execute(ctx.pool())
    .await
    .map_err(|e| AuthError::internal(format!("failed to upsert account: {e}")))?;

    // Generate JWT tokens
    let family_id = primitives::generate_id();
    primitives::generate_token_pair_with_family(
        &user.id,
        &user.roles,
        &config.auth_secret,
        config.token_ttl,
        config.refresh_ttl,
        Some(family_id),
    )
    .map_err(AuthError::internal)
}

#[derive(Debug, Clone, serde::Serialize)]
struct ProviderUserInfo {
    id: Option<String>,
    email: Option<String>,
    name: Option<String>,
    avatar_url: Option<String>,
}

async fn fetch_user_info(provider: &str, access_token: &str) -> Result<ProviderUserInfo, AuthError> {
    let client = reqwest::Client::new();

    match provider {
        "google" => {
            let resp = client
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| AuthError::internal(format!("failed to fetch Google user info: {e}")))?;

            let data: serde_json::Value = resp.json().await
                .map_err(|e| AuthError::internal(format!("failed to parse Google user info: {e}")))?;

            Ok(ProviderUserInfo {
                id: data.get("id").and_then(|v| v.as_str()).map(String::from),
                email: data.get("email").and_then(|v| v.as_str()).map(String::from),
                name: data.get("name").and_then(|v| v.as_str()).map(String::from),
                avatar_url: data.get("picture").and_then(|v| v.as_str()).map(String::from),
            })
        }
        "github" => {
            let resp = client
                .get("https://api.github.com/user")
                .bearer_auth(access_token)
                .header("User-Agent", "rok-api-starter")
                .send()
                .await
                .map_err(|e| AuthError::internal(format!("failed to fetch GitHub user info: {e}")))?;

            let data: serde_json::Value = resp.json().await
                .map_err(|e| AuthError::internal(format!("failed to parse GitHub user info: {e}")))?;

            // GitHub doesn't always return email in /user, fetch from /user/emails
            let email = data.get("email").and_then(|v| v.as_str()).map(String::from);
            let email = match email {
                Some(e) if !e.is_empty() => Some(e),
                _ => {
                    let emails_resp = client
                        .get("https://api.github.com/user/emails")
                        .bearer_auth(access_token)
                        .header("User-Agent", "rok-api-starter")
                        .send()
                        .await
                        .map_err(|e| AuthError::internal(format!("failed to fetch GitHub emails: {e}")))?;
                    let emails: Vec<serde_json::Value> = emails_resp.json().await.unwrap_or_default();
                    emails.iter()
                        .find(|e| e.get("primary").and_then(|v| v.as_bool()).unwrap_or(false))
                        .and_then(|e| e.get("email").and_then(|v| v.as_str()))
                        .map(String::from)
                }
            };

            Ok(ProviderUserInfo {
                id: data.get("id").and_then(|v| v.as_number()).map(|n| n.to_string()),
                email,
                name: data.get("name").and_then(|v| v.as_str()).map(String::from),
                avatar_url: data.get("avatar_url").and_then(|v| v.as_str()).map(String::from),
            })
        }
        _ => Err(AuthError::bad_request(format!("unsupported provider: {provider}"))),
    }
}
