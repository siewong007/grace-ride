-- Roles become data. A role is a named bundle of permissions and a permission is one
-- action on one page: '<page>.<view|create|update|delete>'. The pages and their actions
-- are defined in code (`src/perms.rs`); which role holds which permission lives here
-- so administrators can change it without a release.
--
-- Members keep their roles in users.roles (comma-separated role keys). Every active
-- member also holds the built-in 'member' role without it being listed there.

CREATE TABLE roles (
    key         TEXT PRIMARY KEY CHECK (key ~ '^[a-z][a-z0-9_]{0,31}$'),
    name        TEXT NOT NULL DEFAULT '',        -- shown for custom roles; built-in roles are named by the app
    description TEXT NOT NULL DEFAULT '',
    is_system   BIGINT NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),  -- built-in: cannot be deleted
    created_at  TEXT NOT NULL DEFAULT (to_char(NOW() AT TIME ZONE 'Asia/Kuala_Lumpur', 'YYYY-MM-DD"T"HH24:MI:SS'))
);

CREATE TABLE role_permissions (
    role_key   TEXT NOT NULL REFERENCES roles(key) ON DELETE CASCADE,
    permission TEXT NOT NULL,
    PRIMARY KEY (role_key, permission)
);
CREATE INDEX idx_role_permissions_permission ON role_permissions(permission);

-- Permissions the app has already handed to the built-in roles. On start-up only
-- permissions missing from this list get their defaults, so a new page arrives with
-- sensible access while anything an administrator removed stays removed.
CREATE TABLE permission_catalog (
    permission TEXT PRIMARY KEY
);

-- The never-used normalised copy of users.roles allowed only the five original role
-- names, which custom roles would violate.
DROP TABLE IF EXISTS user_roles;
