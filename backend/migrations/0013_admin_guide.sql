-- 恩典同行 Grace Ride — Admin workflow guides
--
-- The in-app manual for staff: ordered, bilingual steps that deep-link into the
-- admin pages they describe. Content is managed in-app by help-centre staff;
-- changing the manual must not require a deploy.

CREATE TABLE IF NOT EXISTS guide_workflows (
  id         BIGSERIAL PRIMARY KEY,
  slug       TEXT UNIQUE NOT NULL,
  sort_order INT     NOT NULL DEFAULT 0,
  published  BOOLEAN NOT NULL DEFAULT TRUE,
  title_en   TEXT NOT NULL,
  title_zh   TEXT NOT NULL,
  summary_en TEXT NOT NULL DEFAULT '',
  summary_zh TEXT NOT NULL DEFAULT '',
  steps      JSONB NOT NULL DEFAULT '[]',
  updated_at TEXT NOT NULL,
  updated_by BIGINT REFERENCES users(id) ON DELETE SET NULL
);
