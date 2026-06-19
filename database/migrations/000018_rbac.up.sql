CREATE TABLE roles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE permissions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE role_permissions (
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id TEXT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE user_roles (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX idx_user_roles_role_id ON user_roles(role_id);
CREATE INDEX idx_role_permissions_role_id ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_permission_id ON role_permissions(permission_id);

-- Seed default roles
INSERT INTO roles (id, name, description) VALUES
    ('role_user', 'user', 'Default user role'),
    ('role_admin', 'admin', 'Administrator with full access');

-- Seed default permissions
INSERT INTO permissions (id, name, description) VALUES
    ('perm_users_read', 'users.read', 'View user profiles'),
    ('perm_users_write', 'users.write', 'Create and update users'),
    ('perm_users_delete', 'users.delete', 'Delete users'),
    ('perm_auth_admin', 'auth.admin', 'Manage authentication settings'),
    ('perm_roles_manage', 'roles.manage', 'Manage roles and permissions');

-- Assign all permissions to admin role
INSERT INTO role_permissions (role_id, permission_id)
SELECT 'role_admin', id FROM permissions;

-- Assign basic permissions to user role
INSERT INTO role_permissions (role_id, permission_id)
SELECT 'role_user', id FROM permissions WHERE name IN ('users.read');

-- Assign admin role to existing admin users (where roles = 'admin')
INSERT INTO user_roles (user_id, role_id)
SELECT id, 'role_admin' FROM users WHERE roles = 'admin';

-- Assign user role to all other users
INSERT INTO user_roles (user_id, role_id)
SELECT id, 'role_user' FROM users WHERE roles != 'admin' OR roles IS NULL;
