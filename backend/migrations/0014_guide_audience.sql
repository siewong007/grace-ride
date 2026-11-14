ALTER TABLE guide_workflows
  ADD COLUMN audience TEXT NOT NULL DEFAULT 'staff'
  CHECK (audience IN ('member', 'staff'));
