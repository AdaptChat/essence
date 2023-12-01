ALTER TABLE users
    DROP COLUMN IF EXISTS discriminator,
    ADD COLUMN IF NOT EXISTS display_name TEXT;

DROP FUNCTION IF EXISTS generate_discriminator(text);