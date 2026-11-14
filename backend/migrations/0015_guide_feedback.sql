CREATE TABLE guide_feedback (
  guide_id   BIGINT NOT NULL REFERENCES guide_workflows(id) ON DELETE CASCADE,
  user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  helpful    BOOLEAN NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (guide_id, user_id)
);
