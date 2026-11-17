-- =====================================================================
-- 0004: member_roles, one row per member per role they hold.
--
-- `users.roles` (comma-separated keys) is still the column the app writes. This
-- table is kept in step with it by the database, and permission checks read the
-- table, so:
--   • a name with no matching role grants nothing and is never stored here;
--   • deleting a role removes it from every member at once (ON DELETE CASCADE);
--   • a role created later attaches to members whose list already names it.
-- The built-in `member` role is held by everyone and is not listed here.
--
-- Named member_roles because a replayed 0001 drops any table called user_roles.
-- Repeatable, like 0002 (an old history may replay every file; see lib.rs).

CREATE TABLE IF NOT EXISTS member_roles (
    user_id  BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_key TEXT   NOT NULL REFERENCES roles(key) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_key)
);
CREATE INDEX IF NOT EXISTS idx_member_roles_role ON member_roles(role_key);

-- Role keys listed in a users.roles value, as perms::parse_roles reads them.
CREATE OR REPLACE FUNCTION listed_role_keys(listed text) RETURNS text[]
LANGUAGE sql IMMUTABLE AS $$
    SELECT COALESCE(array_agg(DISTINCT k), '{}')
    FROM (SELECT trim(x) AS k FROM unnest(string_to_array(COALESCE(listed, ''), ',')) AS x) s
    WHERE k <> '' AND k <> 'member'
$$;

-- Make one member's rows match their users.roles.
CREATE OR REPLACE FUNCTION sync_member_roles(uid bigint, listed text) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
    keys text[] := listed_role_keys(listed);
BEGIN
    DELETE FROM member_roles WHERE user_id = uid AND NOT (role_key = ANY(keys));
    INSERT INTO member_roles (user_id, role_key)
    SELECT uid, r.key FROM roles r WHERE r.key = ANY(keys)
    ON CONFLICT DO NOTHING;
END
$$;

CREATE OR REPLACE FUNCTION users_sync_member_roles() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    PERFORM sync_member_roles(NEW.id, NEW.roles);
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS trg_users_sync_member_roles ON users;
CREATE TRIGGER trg_users_sync_member_roles
    AFTER INSERT OR UPDATE OF roles ON users
    FOR EACH ROW EXECUTE FUNCTION users_sync_member_roles();

CREATE OR REPLACE FUNCTION roles_attach_members() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO member_roles (user_id, role_key)
    SELECT u.id, NEW.key FROM users u WHERE NEW.key = ANY(listed_role_keys(u.roles))
    ON CONFLICT DO NOTHING;
    RETURN NULL;
END
$$;

DROP TRIGGER IF EXISTS trg_roles_attach_members ON roles;
CREATE TRIGGER trg_roles_attach_members
    AFTER INSERT ON roles
    FOR EACH ROW EXECUTE FUNCTION roles_attach_members();

-- Backfill, and point out names that match no role (they grant nothing).
DO $$
DECLARE
    stray record;
BEGIN
    PERFORM sync_member_roles(id, roles) FROM users;
    FOR stray IN
        SELECT u.id, k FROM users u, unnest(listed_role_keys(u.roles)) AS k
        WHERE NOT EXISTS (SELECT 1 FROM roles r WHERE r.key = k)
    LOOP
        RAISE WARNING 'user % lists role "%", which does not exist; it grants nothing', stray.id, stray.k;
    END LOOP;
END
$$;
