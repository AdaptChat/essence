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
    WHERE NOT EXISTS (
        SELECT discriminator FROM users WHERE username = $1 AND discriminator = result.discrim
    )
    LIMIT 1
    INTO out;
    RETURN out;
END;
$$;