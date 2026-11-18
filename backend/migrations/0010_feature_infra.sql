-- Add infrastructure for remaining features
CREATE TABLE IF NOT EXISTS broadcasts (id SERIAL PRIMARY KEY, title TEXT, body TEXT, created_at TIMESTAMP NOT NULL);
CREATE TABLE IF NOT EXISTS receipts (id SERIAL PRIMARY KEY, ride_id INT, amount DECIMAL, image_path TEXT, created_at TIMESTAMP NOT NULL);
CREATE TABLE IF NOT EXISTS budget_records (id SERIAL PRIMARY KEY, category TEXT, amount DECIMAL, created_at TIMESTAMP NOT NULL);
