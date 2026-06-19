use std::time::Duration;

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub roles: String,
    #[serde(default)]
    pub permissions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_id: Option<String>,
}

impl Claims {
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.split(',').any(|p| p == perm)
    }

    pub fn has_any_permission(&self, perms: &[&str]) -> bool {
        perms.iter().any(|p| self.has_permission(p))
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.split(',').any(|r| r == role)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

pub fn generate_token_pair(
    user_id: &str,
    roles: &str,
    permissions: &str,
    secret: &str,
    token_ttl: Duration,
    refresh_ttl: Duration,
) -> Result<TokenPair, jsonwebtoken::errors::Error> {
    generate_token_pair_with_family(user_id, roles, permissions, secret, token_ttl, refresh_ttl, None)
}

pub fn generate_token_pair_with_family(
    user_id: &str,
    roles: &str,
    permissions: &str,
    secret: &str,
    token_ttl: Duration,
    refresh_ttl: Duration,
    family_id: Option<String>,
) -> Result<TokenPair, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;

    let access_claims = Claims {
        sub: user_id.to_string(),
        exp: now + token_ttl.as_secs() as usize,
        iat: now,
        roles: roles.to_string(),
        permissions: permissions.to_string(),
        family_id: None,
    };

    let refresh_claims = Claims {
        sub: user_id.to_string(),
        exp: now + refresh_ttl.as_secs() as usize,
        iat: now,
        roles: roles.to_string(),
        permissions: permissions.to_string(),
        family_id,
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(TokenPair {
        access_token,
        refresh_token,
    })
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}
