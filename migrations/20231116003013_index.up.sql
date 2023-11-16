-- Add up migration script here
CREATE UNIQUE INDEX IF NOT EXISTS username_index ON users(lower(username));