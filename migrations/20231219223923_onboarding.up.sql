ALTER TABLE users
    ADD COLUMN IF NOT EXISTS onboarding_flags BIGINT NOT NULL DEFAULT 0;