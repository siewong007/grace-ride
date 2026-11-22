-- Optional custom icon per activity/event (preset icon key or short emoji/symbol)
ALTER TABLE events ADD COLUMN IF NOT EXISTS icon TEXT;
