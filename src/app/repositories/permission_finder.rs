use std::sync::Arc;

use di::injectable;
use sqlx::PgPool;

/// Bridges RBAC lookups into `crates/auth`'s `AuthContext` shape. Talks to `role_permissions`
/// directly (there's no `PermissionRepository` abstraction yet — this is the only consumer).
#[injectable]
pub struct AppPermissionFinder {
    #[inject]
    pool: Arc<PgPool>,
}

#[async_trait::async_trait]
impl auth::context::PermissionFinder for AppPermissionFinder {
    async fn get_user_permissions(&self, user_id: &str) -> Result<String, sqlx::Error> {
        let perms: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT p.name FROM user_roles ur
             JOIN role_permissions rp ON rp.role_id = ur.role_id
             JOIN permissions p ON p.id = rp.permission_id
             WHERE ur.user_id = $1
             ORDER BY p.name",
        )
        .bind(user_id)
        .fetch_all(self.pool.as_ref())
        .await?;
        Ok(perms.join(","))
    }
}
