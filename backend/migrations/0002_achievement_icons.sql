-- Custom achievement icons per user (managed by admin)
CREATE TABLE IF NOT EXISTS user_badge_icons (
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    badge_key   TEXT NOT NULL,
    icon        TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (app_now()),
    updated_by  BIGINT REFERENCES users(id) ON DELETE SET NULL,
    PRIMARY KEY (user_id, badge_key)
);
CREATE INDEX IF NOT EXISTS idx_user_badge_icons_user ON user_badge_icons(user_id);
