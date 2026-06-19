use crate::context::AuthContext;
use crate::error::AuthError;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoleWithPermissions {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

// ─── Permission checking ──────────────────────────────────

pub async fn user_has_permission<C: AuthContext>(
    ctx: &C,
    user_id: &str,
    permission: &str,
) -> Result<bool, AuthError> {
    let has: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM user_roles ur
            JOIN role_permissions rp ON rp.role_id = ur.role_id
            JOIN permissions p ON p.id = rp.permission_id
            WHERE ur.user_id = $1 AND p.name = $2
        )",
    )
    .bind(user_id)
    .bind(permission)
    .fetch_one(ctx.pool())
    .await?;
    Ok(has)
}

pub async fn user_permissions<C: AuthContext>(
    ctx: &C,
    user_id: &str,
) -> Result<Vec<String>, AuthError> {
    sqlx::query_scalar(
        "SELECT DISTINCT p.name FROM user_roles ur
         JOIN role_permissions rp ON rp.role_id = ur.role_id
         JOIN permissions p ON p.id = rp.permission_id
         WHERE ur.user_id = $1
         ORDER BY p.name",
    )
    .bind(user_id)
    .fetch_all(ctx.pool())
    .await
    .map_err(AuthError::from)
}

// ─── Role CRUD ────────────────────────────────────────────

pub async fn list_roles<C: AuthContext>(ctx: &C) -> Result<Vec<RoleWithPermissions>, AuthError> {
    let roles: Vec<Role> = sqlx::query_as("SELECT * FROM roles ORDER BY name")
        .fetch_all(ctx.pool())
        .await?;

    let mut result = Vec::new();
    for role in roles {
        let permissions: Vec<String> = sqlx::query_scalar(
            "SELECT p.name FROM role_permissions rp
             JOIN permissions p ON p.id = rp.permission_id
             WHERE rp.role_id = $1
             ORDER BY p.name",
        )
        .bind(&role.id)
        .fetch_all(ctx.pool())
        .await?;

        result.push(RoleWithPermissions {
            id: role.id,
            name: role.name,
            description: role.description,
            permissions,
        });
    }

    Ok(result)
}

pub async fn create_role<C: AuthContext>(
    ctx: &C,
    name: &str,
    description: Option<&str>,
) -> Result<Role, AuthError> {
    let id = crate::primitives::generate_id();
    sqlx::query_as::<_, Role>(
        "INSERT INTO roles (id, name, description) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(&id)
    .bind(name)
    .bind(description)
    .fetch_one(ctx.pool())
    .await
    .map_err(AuthError::from)
}

pub async fn delete_role<C: AuthContext>(ctx: &C, role_id: &str) -> Result<(), AuthError> {
    // Prevent deleting system roles
    if role_id == "role_user" || role_id == "role_admin" {
        return Err(AuthError::bad_request("cannot delete system roles"));
    }

    let affected = sqlx::query("DELETE FROM roles WHERE id = $1 AND name NOT IN ('user', 'admin')")
        .bind(role_id)
        .execute(ctx.pool())
        .await?
        .rows_affected();

    if affected == 0 {
        return Err(AuthError::not_found("role not found or is a system role"));
    }

    Ok(())
}

// ─── Permission management ────────────────────────────────

pub async fn list_permissions<C: AuthContext>(ctx: &C) -> Result<Vec<Permission>, AuthError> {
    sqlx::query_as("SELECT * FROM permissions ORDER BY name")
        .fetch_all(ctx.pool())
        .await
        .map_err(AuthError::from)
}

pub async fn grant_permission_to_role<C: AuthContext>(
    ctx: &C,
    role_id: &str,
    permission_id: &str,
) -> Result<(), AuthError> {
    let _ = sqlx::query(
        "INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(role_id)
    .bind(permission_id)
    .execute(ctx.pool())
    .await?;
    Ok(())
}

pub async fn revoke_permission_from_role<C: AuthContext>(
    ctx: &C,
    role_id: &str,
    permission_id: &str,
) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM role_permissions WHERE role_id = $1 AND permission_id = $2")
        .bind(role_id)
        .bind(permission_id)
        .execute(ctx.pool())
        .await?;
    Ok(())
}

// ─── User role assignment ─────────────────────────────────

pub async fn assign_role_to_user<C: AuthContext>(
    ctx: &C,
    user_id: &str,
    role_id: &str,
) -> Result<(), AuthError> {
    let _ = sqlx::query(
        "INSERT INTO user_roles (user_id, role_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(role_id)
    .execute(ctx.pool())
    .await?;
    Ok(())
}

pub async fn remove_role_from_user<C: AuthContext>(
    ctx: &C,
    user_id: &str,
    role_id: &str,
) -> Result<(), AuthError> {
    // Prevent removing the last admin role
    if role_id == "role_admin" {
        let admin_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_roles WHERE role_id = 'role_admin'",
        )
        .fetch_one(ctx.pool())
        .await?;

        if admin_count <= 1 {
            return Err(AuthError::bad_request("cannot remove the last admin role"));
        }
    }

    sqlx::query("DELETE FROM user_roles WHERE user_id = $1 AND role_id = $2")
        .bind(user_id)
        .bind(role_id)
        .execute(ctx.pool())
        .await?;
    Ok(())
}

pub async fn user_roles<C: AuthContext>(ctx: &C, user_id: &str) -> Result<Vec<Role>, AuthError> {
    sqlx::query_as(
        "SELECT r.* FROM roles r
         JOIN user_roles ur ON ur.role_id = r.id
         WHERE ur.user_id = $1
         ORDER BY r.name",
    )
    .bind(user_id)
    .fetch_all(ctx.pool())
    .await
    .map_err(AuthError::from)
}
