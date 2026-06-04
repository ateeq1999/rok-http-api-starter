use rok_orm::Model;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, Model, rok_orm::Table)]
#[rok_orm(table = "email_verification_tokens", timestamps)]
#[table(name = "email_verification_tokens")]
pub struct EmailVerificationToken {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
