-- Announcements for chosen groups (see `crate::audience`).
--
-- `targeted = 0`: for every member (every announcement published before this, so none
-- needs a backfill). `targeted = 1`: only for the groups in `announcement_groups` — like
-- `event_groups`, members of a listed group or of any group below it, and everyone when
-- the `is_all` group is listed. The flag is kept apart from the rows so that deleting
-- every group an announcement was for hides it rather than opening it to everyone.
ALTER TABLE announcements ADD COLUMN IF NOT EXISTS targeted BIGINT NOT NULL DEFAULT 0;
ALTER TABLE announcements DROP CONSTRAINT IF EXISTS chk_announcements_targeted;
ALTER TABLE announcements ADD CONSTRAINT chk_announcements_targeted CHECK (targeted IN (0, 1));

CREATE TABLE IF NOT EXISTS announcement_groups (
    announcement_id BIGINT NOT NULL REFERENCES announcements(id) ON DELETE CASCADE,
    group_id        BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (announcement_id, group_id)
);

CREATE INDEX IF NOT EXISTS idx_announcement_groups_group ON announcement_groups(group_id);
