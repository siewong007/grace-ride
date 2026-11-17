-- =====================================================================
-- 0005: JSON kept as JSON. Two columns held JSON as plain text:
--   • settlement_revisions.snapshot: the bill as it stood when a revision was saved;
--   • invites.group_ids: the groups an invite adds (always '[]' today).
-- They become JSONB, so the database refuses anything that isn't JSON. The app binds
-- `$n::jsonb` on write and reads `col::text`, as it already does for
-- guide_workflows.steps.
--
-- A stored value that isn't valid JSON is kept as a JSON string, with a warning,
-- rather than failing the deploy.
--
-- Repeatable, like 0002 (an old history may replay every file; see lib.rs): each
-- column is converted only while it is still text.

CREATE OR REPLACE FUNCTION pg_temp.as_jsonb(v text, what text) RETURNS jsonb
LANGUAGE plpgsql AS $$
BEGIN
    RETURN v::jsonb;
EXCEPTION WHEN invalid_text_representation THEN
    RAISE WARNING '% is not JSON; kept as a JSON string: %', what, left(v, 80);
    RETURN to_jsonb(v);
END
$$;

DO $$
BEGIN
    IF (SELECT data_type FROM information_schema.columns
        WHERE table_schema = current_schema() AND table_name = 'settlement_revisions'
          AND column_name = 'snapshot') = 'text' THEN
        ALTER TABLE settlement_revisions
            ALTER COLUMN snapshot TYPE jsonb
            USING pg_temp.as_jsonb(snapshot, 'settlement_revisions.snapshot of revision ' || id);
    END IF;

    IF (SELECT data_type FROM information_schema.columns
        WHERE table_schema = current_schema() AND table_name = 'invites'
          AND column_name = 'group_ids') = 'text' THEN
        ALTER TABLE invites ALTER COLUMN group_ids DROP DEFAULT;
        ALTER TABLE invites
            ALTER COLUMN group_ids TYPE jsonb
            USING pg_temp.as_jsonb(group_ids, 'invites.group_ids of invite ' || id);
        ALTER TABLE invites ALTER COLUMN group_ids SET DEFAULT '[]'::jsonb;
    END IF;
END
$$;

-- An invite's groups are a list of group ids.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'invites'::regclass AND conname = 'chk_invites_group_ids') THEN
        ALTER TABLE invites ADD CONSTRAINT chk_invites_group_ids
            CHECK (jsonb_typeof(group_ids) = 'array') NOT VALID;
    END IF;
    BEGIN
        ALTER TABLE invites VALIDATE CONSTRAINT chk_invites_group_ids;
    EXCEPTION WHEN check_violation THEN
        RAISE WARNING 'rows in invites break chk_invites_group_ids: new rows are checked, existing ones need review';
    END;
END
$$;
