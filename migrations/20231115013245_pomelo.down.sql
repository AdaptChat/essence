CREATE OR REPLACE FUNCTION generate_discriminator(TEXT)
    RETURNS SMALLINT
    LANGUAGE plpgsql
AS $$
DECLARE
    out SMALLINT;
BEGIN
    SELECT * FROM (
      SELECT
          trunc(random() * 9999 + 1) AS discrim
      FROM
          generate_series(1, 9999)
    ) AS result
    WHERE result.discrim NOT IN (
        SELECT discriminator FROM users WHERE username = $1
    )
    LIMIT 1
    INTO out;
    RETURN out;
END;
$$;

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS discriminator SMALLINT NOT NULL DEFAULT generate_discriminator('username');