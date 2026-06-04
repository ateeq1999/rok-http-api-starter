use rok_auth::error::AuthError;
use rok_auth::provider::UserProvider;
use rok_orm::Model;
use rok_orm::PgModel;
use rok_orm::SqlValue;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, rok_orm::Model)]
#[rok_orm(table = "users", timestamps)]
pub struct User {
    pub id: i64,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub roles: String,
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    pub fn role_list(&self) -> Vec<String> {
        self.roles.split(',').map(|s| s.trim().to_string()).collect()
    }

    pub async fn find_by_email(
        email: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        Self::filter("email", SqlValue::Text(email.to_lowercase()))
            .first()
            .await
    }

    pub async fn create_user(
        pool: &sqlx::PgPool,
        email: &str,
        password_hash: &str,
        name: &str,
    ) -> Result<Self, sqlx::Error> {
        Self::create_returning(
            pool,
            &[
                ("email", SqlValue::Text(email.to_lowercase())),
                ("password_hash", SqlValue::Text(password_hash.into())),
                ("name", SqlValue::Text(name.into())),
                ("roles", SqlValue::Text("user".into())),
            ],
        )
        .await
    }
}

impl UserProvider for User {
    type Id = i64;

    fn user_id(&self) -> Self::Id {
        self.id
    }

    fn password_hash(&self) -> &str {
        &self.password_hash
    }

    fn roles(&self) -> Vec<String> {
        self.role_list()
    }

    fn find_by_email(
        pool: &sqlx::PgPool,
        email: &str,
    ) -> impl std::future::Future<Output = Result<Option<Self>, AuthError>> + Send {
        let email = email.to_lowercase();
        async move {
            Self::find_where_explicit(
                pool,
                Self::query().where_eq("email", SqlValue::Text(email)),
            )
            .await
            .map(|mut v| v.pop())
            .map_err(|e| AuthError::Internal(e.to_string()))
        }
    }

    fn find_by_id(
        pool: &sqlx::PgPool,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<Self>, AuthError>> + Send {
        let id = id.to_string();
        async move {
            let id: i64 = id.parse().map_err(|_| AuthError::Internal("invalid user id".into()))?;
            Self::find_by_pk_explicit(pool, id)
                .await
                .map_err(|e| AuthError::Internal(e.to_string()))
        }
    }
}
