ALTER TABLE members
    ADD COLUMN IF NOT EXISTS permissions BIGINT NOT NULL DEFAULT 0;