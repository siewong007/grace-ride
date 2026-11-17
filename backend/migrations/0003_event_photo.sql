-- Optional cover photo on an event.
--
-- A soft reference to files.id, like users.photo_file_id: the API only accepts an
-- image the organiser uploaded themselves, and the file stays readable to everyone
-- who can see the event (see files::can_access_file).
--
-- Safe on any database: the column is added only when missing, so a replay changes
-- nothing.

ALTER TABLE events ADD COLUMN IF NOT EXISTS photo_file_id BIGINT;
