-- 0006 · Record when a person opts in to the terms. NULL means they have not agreed.
ALTER TABLE users ADD COLUMN terms_accepted_at TEXT;
