-- Custom avatar for members without a photo: a background colour and an optional icon,
-- each stored as a key from a fixed list the API checks (never a raw colour or glyph).
-- NULL means the default (initial on the neutral background).
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_color TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_icon TEXT;
