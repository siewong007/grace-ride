-- Help desk: who on staff has taken a request, and an optional photo on any message.
ALTER TABLE help_tickets ADD COLUMN assignee_id BIGINT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE help_replies ADD COLUMN file_id BIGINT REFERENCES files(id) ON DELETE SET NULL;
CREATE INDEX idx_help_replies_file ON help_replies(file_id) WHERE file_id IS NOT NULL;
