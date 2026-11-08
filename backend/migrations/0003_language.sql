-- 0003 · Per-user UI language (backend renders messages in this language).
ALTER TABLE users ADD COLUMN lang TEXT NOT NULL DEFAULT 'zh' CHECK (lang IN ('zh','en'));
